use std::sync::LazyLock;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Catalog {
    pub credits_per_block: u32,
    credit_tiers: Vec<CreditTier>,
    pub games: Vec<Game>,
}

#[derive(Deserialize)]
struct CreditTier {
    under_cents: Option<u32>,
    credits: u8,
}

#[derive(Deserialize)]
pub struct Game {
    pub name: String,
    steam_app_id: Option<u32>,
    price_cents: u32,
    copies: u32,
    #[serde(default)]
    friend_pass: bool,
    image_url: Option<String>,
}

pub static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let mut catalog: Catalog =
        serde_json::from_str(include_str!("games.json")).expect("games.json is invalid");
    let mut games = std::mem::take(&mut catalog.games);
    games.sort_by_cached_key(|g| catalog.credits_for(g));
    catalog.games = games;
    catalog
});

impl Catalog {
    pub fn find(&self, name: &str) -> Option<&Game> {
        self.games.iter().find(|g| g.name == name)
    }

    pub fn credits_for(&self, game: &Game) -> u8 {
        let total = game.price_cents * game.copies;
        self.credit_tiers
            .iter()
            .find(|t| t.under_cents.is_none_or(|under| total < under))
            .unwrap_or_else(|| panic!("No credit tier covers {}", game.name))
            .credits
    }

    pub fn cost(&self, names: &[String]) -> u32 {
        names
            .iter()
            .filter_map(|n| self.find(n))
            .map(|g| u32::from(self.credits_for(g)))
            .sum()
    }

    pub fn budget(&self, blocks: u32) -> u32 {
        self.credits_per_block * blocks.max(1)
    }

    pub fn blocks_needed(&self, cost: u32) -> u32 {
        cost.div_ceil(self.credits_per_block).max(1)
    }
}

pub fn blocks_earned(hours: f32, hours_needed: f32) -> u32 {
    if hours_needed <= 0.0 {
        return 0;
    }
    (hours / hours_needed).floor() as u32
}

impl Game {
    pub fn copies_label(&self) -> String {
        match (self.copies, self.friend_pass) {
            (1, true) => "1 copy + friend pass".into(),
            (1, false) => "1 copy".into(),
            (n, _) => format!("{n} copies"),
        }
    }

    pub fn image_url(&self) -> String {
        match (&self.image_url, self.steam_app_id) {
            (Some(url), _) => url.clone(),
            (None, Some(id)) => format!(
                "https://shared.steamstatic.com/store_item_assets/steam/apps/{id}/header.jpg"
            ),
            (None, None) => panic!("{} has neither a Steam app ID nor an image URL", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CATALOG, blocks_earned};

    #[test]
    fn credits_scale_with_hours() {
        assert_eq!(blocks_earned(0.0, 5.0), 0);
        assert_eq!(blocks_earned(4.99, 5.0), 0);
        assert_eq!(blocks_earned(5.0, 5.0), 1);
        assert_eq!(blocks_earned(14.99, 5.0), 2);
        assert_eq!(blocks_earned(9.0, 3.0), 3);
        assert_eq!(CATALOG.budget(0), 2);
        assert_eq!(CATALOG.budget(1), 2);
        assert_eq!(CATALOG.budget(3), 6);
        assert_eq!(CATALOG.blocks_needed(0), 1);
        assert_eq!(CATALOG.blocks_needed(2), 1);
        assert_eq!(CATALOG.blocks_needed(3), 2);
        assert_eq!(CATALOG.blocks_needed(4), 2);
    }

    #[test]
    fn every_game_has_an_image_and_a_tier() {
        for game in &CATALOG.games {
            assert!(game.image_url().starts_with("https://"), "{}", game.name);
            CATALOG.credits_for(game);
        }
    }

    #[test]
    fn minecraft_is_listed() {
        let game = CATALOG.find("Minecraft").unwrap();
        assert_eq!(CATALOG.credits_for(game), 2);
        assert_eq!(game.copies_label(), "2 copies");
        assert!(game.image_url().contains("minecraft.net"));
    }
}
