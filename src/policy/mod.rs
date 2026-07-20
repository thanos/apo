//! Policy layer: map observational evidence to scores.
//!
//! Category scores average per-rule status weights. The overall score is a
//! weighted average using the v0.1 scoring rubric.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::evidence::{Category, Finding, Status};

/// Score for a single hygiene category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScore {
    pub category: Category,
    pub id: String,
    pub name: String,
    /// Rubric weight as a fraction of the overall score (e.g. `0.20` = 20%).
    pub weight: f64,
    /// Short rubric rationale.
    pub note: String,
    /// 0–100 score (None when all findings are NotApplicable).
    pub score: Option<f64>,
    /// Contribution to overall score when this category participates (`score * renormalized_weight`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weighted_contribution: Option<f64>,
    pub finding_count: usize,
    pub enforced: usize,
    pub present: usize,
    pub partial: usize,
    pub missing: usize,
    pub unknown: usize,
    pub not_applicable: usize,
}

/// Overall policy result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// Overall 0–100 score (weighted rubric).
    pub overall_score: Option<f64>,
    /// Identifier for the scoring rubric applied.
    pub rubric: String,
    pub categories: Vec<CategoryScore>,
    /// Rules with Missing/Partial/Unknown status.
    pub gaps: Vec<String>,
    /// Short recommendations derived from remediations.
    pub recommendations: Vec<String>,
}

/// Numeric weight for a status. Rules produce evidence; policy assigns weight.
fn status_weight(status: Status) -> Option<f64> {
    match status {
        Status::Enforced => Some(100.0),
        Status::Present => Some(80.0),
        Status::Partial => Some(45.0),
        Status::Missing => Some(0.0),
        Status::Unknown => Some(25.0),
        Status::NotApplicable => None,
    }
}

/// Compute category and overall scores from findings.
pub fn evaluate(findings: &[Finding]) -> PolicyResult {
    let mut by_cat: BTreeMap<Category, Vec<&Finding>> = BTreeMap::new();
    for f in findings {
        by_cat.entry(f.category).or_default().push(f);
    }

    let mut categories = Vec::new();
    let mut gap_rules = Vec::new();
    let mut recommendations = Vec::new();

    for &cat in Category::all() {
        let list = by_cat.get(&cat).cloned().unwrap_or_default();
        let mut score_sum = 0.0;
        let mut score_n = 0usize;
        let mut enforced = 0;
        let mut present = 0;
        let mut partial = 0;
        let mut missing = 0;
        let mut unknown = 0;
        let mut not_applicable = 0;

        for f in &list {
            match f.status {
                Status::Enforced => enforced += 1,
                Status::Present => present += 1,
                Status::Partial => partial += 1,
                Status::Missing => missing += 1,
                Status::Unknown => unknown += 1,
                Status::NotApplicable => not_applicable += 1,
            }
            if let Some(w) = status_weight(f.status) {
                score_sum += w;
                score_n += 1;
            }
            if f.status.is_gap() {
                gap_rules.push(f.rule.clone());
                if let Some(r) = &f.remediation {
                    recommendations.push(format!("{}: {r}", f.rule));
                }
            }
        }

        categories.push(CategoryScore {
            category: cat,
            id: cat.id().to_string(),
            name: cat.display_name().to_string(),
            weight: cat.weight(),
            note: cat.rubric_note().to_string(),
            score: if score_n == 0 {
                None
            } else {
                Some(score_sum / score_n as f64)
            },
            weighted_contribution: None,
            finding_count: list.len(),
            enforced,
            present,
            partial,
            missing,
            unknown,
            not_applicable,
        });
    }

    gap_rules.sort();
    gap_rules.dedup();
    recommendations.sort();
    recommendations.dedup();

    // Weighted overall: renormalize rubric weights across categories that have a score.
    let active_weight: f64 = categories
        .iter()
        .filter(|c| c.score.is_some())
        .map(|c| c.weight)
        .sum();

    let mut overall = None;
    if active_weight > 0.0 {
        let mut sum = 0.0;
        for c in &mut categories {
            if let Some(score) = c.score {
                let contrib = score * (c.weight / active_weight);
                c.weighted_contribution = Some(contrib);
                sum += contrib;
            }
        }
        overall = Some(sum);
    }

    PolicyResult {
        overall_score: overall,
        rubric: "apo-hygiene-v0.1".into(),
        categories,
        gaps: gap_rules,
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Confidence, Finding, Status};

    fn finding(rule: &str, category: Category, status: Status) -> Finding {
        Finding::builder(rule, category)
            .status(status)
            .confidence(Confidence::High)
            .summary("test")
            .build()
    }

    #[test]
    fn scores_enforced_higher_than_missing() {
        let findings = vec![
            finding("a", Category::Documentation, Status::Enforced),
            finding("b", Category::Documentation, Status::Missing),
        ];
        let policy = evaluate(&findings);
        let doc = policy
            .categories
            .iter()
            .find(|c| c.category == Category::Documentation)
            .unwrap();
        assert!((doc.score.unwrap() - 50.0).abs() < f64::EPSILON);
        assert!((doc.weight - 0.20).abs() < f64::EPSILON);
    }

    #[test]
    fn overall_uses_category_rubric_weights() {
        // Documentation 100 @ 20%, Testing 0 @ 20%, all others absent →
        // only those two score; renormalize 0.20+0.20 → equal 50/50 → overall 50.
        let findings = vec![
            finding("doc", Category::Documentation, Status::Enforced),
            finding("qa", Category::Testing, Status::Missing),
        ];
        let policy = evaluate(&findings);
        let overall = policy.overall_score.unwrap();
        assert!(
            (overall - 50.0).abs() < 1e-9,
            "expected 50.0, got {overall}"
        );

        // With all six categories Present (80): overall must be 80.
        let all_present: Vec<_> = Category::all()
            .iter()
            .enumerate()
            .map(|(i, &cat)| finding(&format!("r{i}"), cat, Status::Present))
            .collect();
        let policy = evaluate(&all_present);
        assert!((policy.overall_score.unwrap() - 80.0).abs() < 1e-9);

        // Documentation Enforced (100), everything else Missing (0):
        // 100*0.20 + 0*0.80 = 20
        let mut mixed: Vec<_> = Category::all()
            .iter()
            .enumerate()
            .map(|(i, &cat)| finding(&format!("m{i}"), cat, Status::Missing))
            .collect();
        mixed[0] = finding("doc", Category::Documentation, Status::Enforced);
        let policy = evaluate(&mixed);
        assert!(
            (policy.overall_score.unwrap() - 20.0).abs() < 1e-9,
            "expected 20.0, got {}",
            policy.overall_score.unwrap()
        );
    }

    #[test]
    fn rubric_weights_sum_to_one() {
        let sum: f64 = Category::all().iter().map(|c| c.weight()).sum();
        assert!((sum - 1.0).abs() < 1e-12);
    }
}
