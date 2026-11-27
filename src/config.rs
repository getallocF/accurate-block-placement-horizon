use std::fs;
use std::path::Path;

use mod_api::prelude::*;

pub const MOD_ID: &str = "accurateblockplacement";

pub const KEY_ACCURATE_PLACE: &str = "accurateplace-enabled";
pub const KEY_FAST_BREAK: &str = "fastbreak-enabled";
pub const KEY_CONFIRMATION: &str = "confirmation";
pub const KEY_CONFIRMATION_TYPE: &str = "confirmation-type";
pub const KEY_TOOLS: &str = "tools-enabled";
pub const KEY_BUCKETS: &str = "bucket-enabled";
pub const KEY_ARMOR_STANDS: &str = "armorstand-enabled";
pub const KEY_ITEM_FRAMES: &str = "itemframe-enabled";
pub const KEY_SPAWN_EGGS: &str = "spawneggs-enabled";

pub const SECTION_ITEM_GROUPS: &str = "text.autoconfig.accurateblockplacement.section.itemGroups";
pub const CHOICE_CHAT_LABEL: &str =
    "text.autoconfig.accurateblockplacement.option.confirmationType.CHAT";
pub const CHOICE_HUD_LABEL: &str =
    "text.autoconfig.accurateblockplacement.option.confirmationType.HUD";

pub const CONFIG_FILE_NAME: &str = "accurateblockplacement.toml";
pub const LEGACY_PROPERTIES_FILE_NAME: &str = "accurateblockplacement.properties";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationTarget {
    Chat = 0,
    Hud = 1,
}

impl ConfirmationTarget {
    pub fn from_choice_index(index: usize) -> Self {
        match index {
            0 => Self::Chat,
            _ => Self::Hud,
        }
    }

    pub fn as_choice_index(self) -> usize {
        match self {
            Self::Chat => 0,
            Self::Hud => 1,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "CHAT",
            Self::Hud => "HUD",
        }
    }

    pub fn parse_str(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().trim_matches('"');
        if trimmed.eq_ignore_ascii_case("HUD") || trimmed.eq_ignore_ascii_case("true") {
            Some(Self::Hud)
        } else if trimmed.eq_ignore_ascii_case("CHAT") || trimmed.eq_ignore_ascii_case("false") {
            Some(Self::Chat)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbpConfig {
    pub accurate_placement_enabled: bool,
    pub fast_breaking_enabled: bool,
    pub confirmation: bool,
    pub confirmation_type: ConfirmationTarget,
    pub tools_enabled: bool,
    pub bucket_enabled: bool,
    pub armor_stand_enabled: bool,
    pub item_frame_enabled: bool,
    pub spawn_eggs_enabled: bool,
}

impl Default for AbpConfig {
    fn default() -> Self {
        Self {
            accurate_placement_enabled: true,
            fast_breaking_enabled: false,
            confirmation: true,
            confirmation_type: ConfirmationTarget::Hud,
            tools_enabled: true,
            bucket_enabled: true,
            armor_stand_enabled: true,
            item_frame_enabled: true,
            spawn_eggs_enabled: true,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct SettingKeys {
    pub accurate_place: Option<SettingKey>,
    pub fast_break: Option<SettingKey>,
    pub confirmation: Option<SettingKey>,
    pub confirmation_type: Option<SettingKey>,
    pub tools: Option<SettingKey>,
    pub buckets: Option<SettingKey>,
    pub armor_stands: Option<SettingKey>,
    pub item_frames: Option<SettingKey>,
    pub spawn_eggs: Option<SettingKey>,
}

impl SettingKeys {
    pub fn register_all(r: &mut dyn Registrar) -> Self {
        let accurate_place = r
            .register_setting(SettingSpec::toggle(
                KEY_ACCURATE_PLACE,
                "text.autoconfig.accurateblockplacement.option.accuratePlacementEnabled",
                true,
            ))
            .ok();

        let fast_break = r
            .register_setting(SettingSpec::toggle(
                KEY_FAST_BREAK,
                "text.autoconfig.accurateblockplacement.option.fastBreakingEnabled",
                false,
            ))
            .ok();

        let confirmation = r
            .register_setting(SettingSpec::toggle(
                KEY_CONFIRMATION,
                "text.autoconfig.accurateblockplacement.option.confirmation",
                true,
            ))
            .ok();

        let confirmation_type = r
            .register_setting(SettingSpec::choice(
                KEY_CONFIRMATION_TYPE,
                "text.autoconfig.accurateblockplacement.option.confirmationType",
                [CHOICE_CHAT_LABEL, CHOICE_HUD_LABEL],
                ConfirmationTarget::Hud.as_choice_index(),
            ))
            .ok();

        let tools = r
            .register_setting(
                SettingSpec::toggle(
                    KEY_TOOLS,
                    "text.autoconfig.accurateblockplacement.option.toolsEnabled",
                    true,
                )
                .tooltip("text.autoconfig.accurateblockplacement.option.toolsEnabled.tooltip"),
            )
            .ok();
        let _ = r.categorize_setting(KEY_TOOLS, SECTION_ITEM_GROUPS);

        let buckets = r
            .register_setting(
                SettingSpec::toggle(
                    KEY_BUCKETS,
                    "text.autoconfig.accurateblockplacement.option.bucketEnabled",
                    true,
                )
                .tooltip("text.autoconfig.accurateblockplacement.option.bucketEnabled.tooltip"),
            )
            .ok();
        let _ = r.categorize_setting(KEY_BUCKETS, SECTION_ITEM_GROUPS);

        let armor_stands = r
            .register_setting(
                SettingSpec::toggle(
                    KEY_ARMOR_STANDS,
                    "text.autoconfig.accurateblockplacement.option.armorStandEnabled",
                    true,
                )
                .tooltip("text.autoconfig.accurateblockplacement.option.armorStandEnabled.tooltip"),
            )
            .ok();
        let _ = r.categorize_setting(KEY_ARMOR_STANDS, SECTION_ITEM_GROUPS);

        let item_frames = r
            .register_setting(
                SettingSpec::toggle(
                    KEY_ITEM_FRAMES,
                    "text.autoconfig.accurateblockplacement.option.itemFrameEnabled",
                    true,
                )
                .tooltip("text.autoconfig.accurateblockplacement.option.itemFrameEnabled.tooltip"),
            )
            .ok();
        let _ = r.categorize_setting(KEY_ITEM_FRAMES, SECTION_ITEM_GROUPS);

        let spawn_eggs = r
            .register_setting(
                SettingSpec::toggle(
                    KEY_SPAWN_EGGS,
                    "text.autoconfig.accurateblockplacement.option.spawnEggsEnabled",
                    true,
                )
                .tooltip("text.autoconfig.accurateblockplacement.option.spawnEggsEnabled.tooltip"),
            )
            .ok();
        let _ = r.categorize_setting(KEY_SPAWN_EGGS, SECTION_ITEM_GROUPS);

        Self {
            accurate_place,
            fast_break,
            confirmation,
            confirmation_type,
            tools,
            buckets,
            armor_stands,
            item_frames,
            spawn_eggs,
        }
    }
}

impl AbpConfig {
    pub fn sync_from_host(&mut self, keys: &SettingKeys, host: &dyn ClientHost) {
        if let Some(k) = keys.accurate_place
            && let Ok(v) = host.setting_bool(k)
        {
            self.accurate_placement_enabled = v;
        }
        if let Some(k) = keys.fast_break
            && let Ok(v) = host.setting_bool(k)
        {
            self.fast_breaking_enabled = v;
        }
        if let Some(k) = keys.confirmation
            && let Ok(v) = host.setting_bool(k)
        {
            self.confirmation = v;
        }
        if let Some(k) = keys.confirmation_type
            && let Ok(idx) = host.setting_choice(k)
        {
            self.confirmation_type = ConfirmationTarget::from_choice_index(idx as usize);
        }
        if let Some(k) = keys.tools
            && let Ok(v) = host.setting_bool(k)
        {
            self.tools_enabled = v;
        }
        if let Some(k) = keys.buckets
            && let Ok(v) = host.setting_bool(k)
        {
            self.bucket_enabled = v;
        }
        if let Some(k) = keys.armor_stands
            && let Ok(v) = host.setting_bool(k)
        {
            self.armor_stand_enabled = v;
        }
        if let Some(k) = keys.item_frames
            && let Ok(v) = host.setting_bool(k)
        {
            self.item_frame_enabled = v;
        }
        if let Some(k) = keys.spawn_eggs
            && let Ok(v) = host.setting_bool(k)
        {
            self.spawn_eggs_enabled = v;
        }
    }

    pub fn push_all_to_host(&self, keys: &SettingKeys, host: &mut dyn ClientHost) {
        if let Some(k) = keys.accurate_place {
            let _ = host.set_setting(k, SettingValue::Toggle(self.accurate_placement_enabled));
        }
        if let Some(k) = keys.fast_break {
            let _ = host.set_setting(k, SettingValue::Toggle(self.fast_breaking_enabled));
        }
        if let Some(k) = keys.confirmation {
            let _ = host.set_setting(k, SettingValue::Toggle(self.confirmation));
        }
        if let Some(k) = keys.confirmation_type {
            let _ = host.set_setting(
                k,
                SettingValue::Choice(self.confirmation_type.as_choice_index()),
            );
        }
        if let Some(k) = keys.tools {
            let _ = host.set_setting(k, SettingValue::Toggle(self.tools_enabled));
        }
        if let Some(k) = keys.buckets {
            let _ = host.set_setting(k, SettingValue::Toggle(self.bucket_enabled));
        }
        if let Some(k) = keys.armor_stands {
            let _ = host.set_setting(k, SettingValue::Toggle(self.armor_stand_enabled));
        }
        if let Some(k) = keys.item_frames {
            let _ = host.set_setting(k, SettingValue::Toggle(self.item_frame_enabled));
        }
        if let Some(k) = keys.spawn_eggs {
            let _ = host.set_setting(k, SettingValue::Toggle(self.spawn_eggs_enabled));
        }
    }

    pub fn apply_setting_change(&mut self, key: &str, value: &SettingValue) -> bool {
        match (key, value) {
            (KEY_ACCURATE_PLACE, SettingValue::Toggle(v)) => {
                self.accurate_placement_enabled = *v;
                true
            }
            (KEY_FAST_BREAK, SettingValue::Toggle(v)) => {
                self.fast_breaking_enabled = *v;
                true
            }
            (KEY_CONFIRMATION, SettingValue::Toggle(v)) => {
                self.confirmation = *v;
                true
            }
            (KEY_CONFIRMATION_TYPE, SettingValue::Choice(idx)) => {
                self.confirmation_type = ConfirmationTarget::from_choice_index(*idx);
                true
            }
            (KEY_TOOLS, SettingValue::Toggle(v)) => {
                self.tools_enabled = *v;
                true
            }
            (KEY_BUCKETS, SettingValue::Toggle(v)) => {
                self.bucket_enabled = *v;
                true
            }
            (KEY_ARMOR_STANDS, SettingValue::Toggle(v)) => {
                self.armor_stand_enabled = *v;
                true
            }
            (KEY_ITEM_FRAMES, SettingValue::Toggle(v)) => {
                self.item_frame_enabled = *v;
                true
            }
            (KEY_SPAWN_EGGS, SettingValue::Toggle(v)) => {
                self.spawn_eggs_enabled = *v;
                true
            }
            _ => false,
        }
    }

    pub fn load_from_config_dir(config_dir: &Path) -> Option<Self> {
        let toml_path = config_dir.join(CONFIG_FILE_NAME);
        if let Ok(contents) = fs::read_to_string(&toml_path) {
            return Some(Self::parse_key_value_text(&contents));
        }

        let legacy_path = config_dir.join(LEGACY_PROPERTIES_FILE_NAME);
        if let Ok(contents) = fs::read_to_string(&legacy_path) {
            return Some(Self::parse_key_value_text(&contents));
        }

        None
    }

    pub fn save_to_config_dir(&self, config_dir: &Path) {
        if fs::create_dir_all(config_dir).is_err() {
            return;
        }
        let toml_path = config_dir.join(CONFIG_FILE_NAME);
        let serialized = self.to_toml_string();
        let _ = fs::write(toml_path, serialized);
    }

    pub fn parse_key_value_text(text: &str) -> Self {
        let mut cfg = Self::default();
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            let Some((raw_key, raw_val)) = line.split_once('=') else {
                continue;
            };
            let key = raw_key.trim();
            let val = raw_val.trim();
            let parse_bool = |s: &str| -> Option<bool> {
                if s.eq_ignore_ascii_case("true") {
                    Some(true)
                } else if s.eq_ignore_ascii_case("false") {
                    Some(false)
                } else {
                    None
                }
            };

            match key {
                KEY_ACCURATE_PLACE => {
                    if let Some(b) = parse_bool(val) {
                        cfg.accurate_placement_enabled = b;
                    }
                }
                KEY_FAST_BREAK => {
                    if let Some(b) = parse_bool(val) {
                        cfg.fast_breaking_enabled = b;
                    }
                }
                KEY_CONFIRMATION => {
                    if let Some(b) = parse_bool(val) {
                        cfg.confirmation = b;
                    }
                }
                KEY_CONFIRMATION_TYPE | "confirmationType" => {
                    if let Some(t) = ConfirmationTarget::parse_str(val) {
                        cfg.confirmation_type = t;
                    }
                }
                KEY_TOOLS => {
                    if let Some(b) = parse_bool(val) {
                        cfg.tools_enabled = b;
                    }
                }
                KEY_BUCKETS => {
                    if let Some(b) = parse_bool(val) {
                        cfg.bucket_enabled = b;
                    }
                }
                KEY_ARMOR_STANDS => {
                    if let Some(b) = parse_bool(val) {
                        cfg.armor_stand_enabled = b;
                    }
                }
                KEY_ITEM_FRAMES => {
                    if let Some(b) = parse_bool(val) {
                        cfg.item_frame_enabled = b;
                    }
                }
                KEY_SPAWN_EGGS => {
                    if let Some(b) = parse_bool(val) {
                        cfg.spawn_eggs_enabled = b;
                    }
                }
                _ => {}
            }
        }
        cfg
    }

    pub fn to_toml_string(&self) -> String {
        format!(
            "# Accurate Block Placement configuration\n\
             {KEY_ACCURATE_PLACE} = {}\n\
             {KEY_FAST_BREAK} = {}\n\
             {KEY_CONFIRMATION} = {}\n\
             {KEY_CONFIRMATION_TYPE} = \"{}\"\n\
             {KEY_TOOLS} = {}\n\
             {KEY_BUCKETS} = {}\n\
             {KEY_ARMOR_STANDS} = {}\n\
             {KEY_ITEM_FRAMES} = {}\n\
             {KEY_SPAWN_EGGS} = {}\n",
            self.accurate_placement_enabled,
            self.fast_breaking_enabled,
            self.confirmation,
            self.confirmation_type.as_str(),
            self.tools_enabled,
            self.bucket_enabled,
            self.armor_stand_enabled,
            self.item_frame_enabled,
            self.spawn_eggs_enabled,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_properties_and_toml_roundtrip() {
        let legacy = "\
            # legacy Fabric properties\n\
            accurateplace-enabled=false\n\
            fastbreak-enabled=true\n\
            confirmation=true\n\
            confirmationType=false\n\
            tools-enabled=false\n\
            bucket-enabled=true\n\
            armorstand-enabled=false\n\
            itemframe-enabled=true\n\
            spawneggs-enabled=false\n";

        let parsed = AbpConfig::parse_key_value_text(legacy);
        assert!(!parsed.accurate_placement_enabled);
        assert!(parsed.fast_breaking_enabled);
        assert!(parsed.confirmation);
        assert_eq!(parsed.confirmation_type, ConfirmationTarget::Chat);
        assert!(!parsed.tools_enabled);
        assert!(parsed.bucket_enabled);
        assert!(!parsed.armor_stand_enabled);
        assert!(parsed.item_frame_enabled);
        assert!(!parsed.spawn_eggs_enabled);

        let serialized = parsed.to_toml_string();
        let reparsed = AbpConfig::parse_key_value_text(&serialized);
        assert_eq!(parsed, reparsed);
    }
}