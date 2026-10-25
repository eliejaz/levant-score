use std::sync::OnceLock;

use super::domain::Preset;

fn canonical() -> &'static Vec<Preset> {
    static PRESETS: OnceLock<Vec<Preset>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        let mut presets: Vec<Preset> = serde_json::from_str(levant_score_rules::PRESETS_JSON)
            .expect("embedded game presets must be valid");
        for preset in &mut presets {
            preset.min_players = *preset
                .allowed_player_counts
                .first()
                .expect("preset player counts must not be empty");
            preset.max_players = *preset
                .allowed_player_counts
                .last()
                .expect("preset player counts must not be empty");
        }
        presets
    })
}

pub fn all() -> Vec<Preset> {
    canonical().clone()
}

pub fn get(code: &str) -> Preset {
    canonical()
        .iter()
        .find(|preset| preset.code == code)
        .or_else(|| canonical().last())
        .expect("at least the custom preset exists")
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_player_counts_match_contract() {
        let expected = [
            ("leekha-solo", vec![4]),
            ("leekha-team", vec![4]),
            ("400", vec![4]),
            ("tarneeb", vec![4]),
            ("chinese", vec![2, 3, 4]),
            ("trex", vec![4]),
            ("trex-complex", vec![4]),
            ("belote", vec![4]),
            ("basra", vec![2, 4]),
            ("hand", vec![2, 3, 4]),
            ("custom", (2..=12).collect()),
        ];
        for (code, counts) in expected {
            assert_eq!(get(code).allowed_player_counts, counts);
        }
    }

    #[test]
    fn tarneeb_target_is_31() {
        assert_eq!(get("tarneeb").default_target, Some(31));
    }
}
