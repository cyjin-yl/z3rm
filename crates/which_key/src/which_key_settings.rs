use settings::{RegisterSetting, Settings, SettingsContent};

#[derive(Debug, Clone, Copy, RegisterSetting)]
pub struct WhichKeySettings {
    pub enabled: bool,
    pub delay_ms: u64,
}

impl Settings for WhichKeySettings {
    fn from_settings(_content: &SettingsContent) -> Self {
        // which_key 设置已从 SettingsContent 中移除 (spec §16 Plan 16)
        // 返回默认值
        Self {
            enabled: false,
            delay_ms: 1000,
        }
    }
}
