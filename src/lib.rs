mod config;
mod filters;
mod placement;

use std::path::PathBuf;
use std::sync::Mutex;

use mod_api::prelude::*;

use crate::config::{AbpConfig, ConfirmationTarget, MOD_ID, SettingKeys};
use crate::placement::{PickAction, PlacementState, compute_mouse_ratio, offset_pos};

const KEY_CATEGORY: &str = "key.category.accurateblockplacement.category";
const KEY_ID_TOGGLE_PLACEMENT: &str = "togglevanillaplacement";
const KEY_ID_TOGGLE_FAST_BREAK: &str = "togglefastbreaking";

const LABEL_TOGGLE_PLACEMENT: &str = "net.clayborn.accurateblockplacement.togglevanillaplacement";
const LABEL_TOGGLE_FAST_BREAK: &str = "net.clayborn.accurateblockplacement.togglefastbreaking";

const MSG_PLACEMENT_ENABLED: &str = "net.clayborn.accurateblockplacement.modplacementmodemessage";
const MSG_PLACEMENT_DISABLED: &str =
    "net.clayborn.accurateblockplacement.vanillaplacementmodemessage";
const MSG_FAST_BREAK_ENABLED: &str = "net.clayborn.accurateblockplacement.fastbreakingenabled";
const MSG_FAST_BREAK_DISABLED: &str = "net.clayborn.accurateblockplacement.fastbreakingdisabled";

#[derive(Default)]
struct ModState {
    config: AbpConfig,
    setting_keys: SettingKeys,
    settings_synced: bool,
    config_dir: Option<PathBuf>,
    loaded_local_config: bool,
    use_key: Option<VanillaKeyMappingRef>,
    attack_key: Option<VanillaKeyMappingRef>,
    toggle_placement_key: Option<ModKeyMappingRef>,
    toggle_fast_break_key: Option<ModKeyMappingRef>,
    placement: PlacementState,
}

#[derive(Default)]
pub struct AccurateBlockPlacementMod {
    state: Mutex<ModState>,
}

impl AccurateBlockPlacementMod {
    #[inline]
    fn with_state<R>(&self, f: impl FnOnce(&mut ModState) -> R) -> R {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard)
    }

    fn send_confirmation_message(
        host: &mut dyn ClientHost,
        cfg: &AbpConfig,
        translation_key: &'static str,
    ) {
        if !cfg.confirmation {
            return;
        }
        let message = Text::translate(translation_key, Vec::new());
        match cfg.confirmation_type {
            ConfirmationTarget::Hud => {
                let _ = host.set_overlay_message(message, false);
            }
            ConfirmationTarget::Chat => {
                let _ = host.chat_line(message);
            }
        }
    }

    fn check_and_confirm_pending_placement(&self, host: &dyn ClientHost) {
        let pending_pos = self.with_state(|st| st.placement.pending_placement.map(|p| p.target_pos));
        let Some(target_pos) = pending_pos else {
            return;
        };

        let current_block = host
            .local_player()
            .and_then(|player| player.level(host))
            .and_then(|level| level.block_state(host, [target_pos.x, target_pos.y, target_pos.z]))
            .map(|state| state.block());

        self.with_state(|st| st.placement.confirm_pending_placement(current_block));
    }

    fn resolve_active_hand_item(
        host: &dyn ClientHost,
        living: LivingRef,
        cfg: &AbpConfig,
    ) -> Option<(Hand, ItemKey)> {
        for hand in Hand::BOTH {
            if let Some(item) = living.with_hand_item(host, hand, |stack| stack.item())
                && filters::is_item_allowed(host, item, cfg)
            {
                return Some((hand, item));
            }
        }
        None
    }

    fn other_hand(hand: Hand) -> Hand {
        match hand {
            Hand::Main => Hand::Off,
            Hand::Off => Hand::Main,
        }
    }
}

#[mod_api::horizon_mod]
impl Hooks for AccurateBlockPlacementMod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let keys = SettingKeys::register_all(r);

        r.register_key_mapping(
            KeyRegistration::new(KEY_ID_TOGGLE_PLACEMENT, "key.keyboard.unknown")
                .label(LABEL_TOGGLE_PLACEMENT)
                .category(KEY_CATEGORY)
                .context(KeyConflictContext::InGame),
        );

        r.register_key_mapping(
            KeyRegistration::new(KEY_ID_TOGGLE_FAST_BREAK, "key.keyboard.unknown")
                .label(LABEL_TOGGLE_FAST_BREAK)
                .category(KEY_CATEGORY)
                .context(KeyConflictContext::InGame),
        );

        self.with_state(|st| {
            st.setting_keys = keys;
        });
    }

    fn client_setup(&self, context: &mut dyn ClientSetupContext, _event: &mut ClientSetupEvent) {
        let use_key = context.vanilla_key_mapping("key.use").ok();
        let attack_key = context.vanilla_key_mapping("key.attack").ok();
        let toggle_placement_key = context.mod_key_mapping(MOD_ID, KEY_ID_TOGGLE_PLACEMENT).ok();
        let toggle_fast_break_key = context
            .mod_key_mapping(MOD_ID, KEY_ID_TOGGLE_FAST_BREAK)
            .ok();

        let config_dir = context.config_dir().to_path_buf();
        let loaded_local = AbpConfig::load_from_config_dir(&config_dir);

        self.with_state(|st| {
            st.use_key = use_key;
            st.attack_key = attack_key;
            st.toggle_placement_key = toggle_placement_key;
            st.toggle_fast_break_key = toggle_fast_break_key;
            st.config_dir = Some(config_dir);
            if let Some(local_cfg) = loaded_local {
                st.config = local_cfg;
                st.loaded_local_config = true;
            }
            st.settings_synced = false;
            st.placement.reset_all();
        });
    }

    fn setting_changed(&self, _host: &dyn Host, event: &mut SettingChangedEvent<'_>) {
        if event.mod_id != MOD_ID {
            return;
        }

        let to_save = self.with_state(|st| {
            let changed = st.config.apply_setting_change(event.key, &event.value);
            if !st.config.accurate_placement_enabled {
                st.placement.reset_all();
            }
            if changed {
                st.config_dir.clone().map(|dir| (dir, st.config.clone()))
            } else {
                None
            }
        });

        if let Some((dir, cfg)) = to_save {
            cfg.save_to_config_dir(&dir);
        }
    }

    fn key_mapping_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut KeyMappingPreEvent<'_>,
    ) -> Flow {
        if event.mapping != "key.use" {
            return Flow::Continue;
        }

        let mouse_ratio = compute_mouse_ratio(host.cursor(), host.window_size());
        self.with_state(|st| {
            if !st.config.accurate_placement_enabled {
                return;
            }
            if event.pressed {
                st.placement.mark_fresh_use_press(mouse_ratio);
            } else {
                st.placement.mark_use_released();
            }
        });

        Flow::Continue
    }

    fn client_pick(&self, host: &mut dyn ClientHost, event: &mut ClientPickEvent<'_>) {
        // Konfirmasi apakah penempatan dari tick sebelumnya benar-benar mengubah blok di dunia.
        self.check_and_confirm_pending_placement(&*host);

        let (cfg, use_key) = self.with_state(|st| (st.config.clone(), st.use_key));

        if !cfg.accurate_placement_enabled || !host.playing() {
            self.with_state(|st| st.placement.reset_all());
            return;
        }

        // Jika pemain sedang menggunakan item aktif (makan, tarik busur, perisai, dll.), biarkan vanilla.
        if host.local_item_use().is_some() {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        let Some(player) = host.local_player() else {
            self.with_state(|st| st.placement.reset_all());
            return;
        };

        let entity = player.entity();
        let Some(living) = host.living(entity) else {
            self.with_state(|st| st.placement.reset_all());
            return;
        };

        // Hanya aktif jika crosshair mengenai blok.
        let current_target = *event.result;
        let Some(hit_target @ CrosshairTarget::Block { pos: hit_pos, state: hit_state, hit }) =
            current_target
        else {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        };

        // Cari tangan pertama (Main lalu Off) yang memegang item yang diizinkan.
        let Some((active_hand, active_item)) = Self::resolve_active_hand_item(&*host, living, &cfg)
        else {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        };

        let active_item_id = host.item_key_id(active_item).unwrap_or("");
        let hit_block_id = host.block_key_id(hit_state.block()).unwrap_or("");
        let is_sneaking = entity.is_shift_key_down(&*host).unwrap_or(false);

        let use_key_down = use_key
            .and_then(|k| k.is_down(&*host).ok())
            .unwrap_or(false);
        let fresh_press_pending = self.with_state(|st| st.placement.fresh_use_press_pending);

        // 1. Jika item di tangan lain memiliki override `use` dan sedang digunakan, lewati.
        let other_hand = Self::other_hand(active_hand);
        if let Some(other_item) = living.with_hand_item(&*host, other_hand, |stack| stack.item()) {
            let other_id = host.item_key_id(other_item).unwrap_or("");
            if !filters::is_item_allowed(&*host, other_item, &cfg)
                && filters::does_item_have_overridden_use(other_id)
                && host.local_item_use().is_some()
            {
                self.with_state(|st| {
                    st.placement.disable_normal_item_use = false;
                });
                return;
            }
        }

        // 2. Blok aktivabel (pintu, tombol, tuas, meja kerja, dll.) kecuali tangga & kecuali sneak.
        if filters::is_activatable_block(hit_block_id)
            && !filters::is_stair_block(hit_block_id)
            && !is_sneaking
        {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        // 3. Container / BaseEntityBlock & Bed saat tombol klik kanan ditahan (bukan klik segar).
        if filters::is_storage_or_bed_block(hit_block_id) && !fresh_press_pending && use_key_down {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        // 4. Scaffolding ke scaffolding tetap menggunakan mekanik tumpuk vanilla.
        if filters::is_scaffolding(active_item_id) && filters::is_scaffolding(hit_block_id) {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        // 5. Composter + item kompos tetap menggunakan vanilla.
        if filters::is_composter_block(hit_block_id) && filters::is_compostable_item(active_item_id)
        {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        // 6. Bone meal di tangan utama ke blok bonemealable tetap menggunakan vanilla.
        if active_hand == Hand::Main
            && filters::is_bone_meal_item(active_item_id)
            && filters::is_bonemealable_block(hit_block_id)
        {
            self.with_state(|st| {
                st.placement.disable_normal_item_use = false;
            });
            return;
        }

        let Some(player_pos) = entity.position(&*host) else {
            return;
        };

        let target_placement_pos = if filters::is_replaceable_block(hit_block_id) {
            hit_pos
        } else {
            offset_pos(hit_pos, hit.face)
        };

        let old_target_block = player
            .level(&*host)
            .and_then(|level| {
                level.block_state(
                    &*host,
                    [
                        target_placement_pos.x,
                        target_placement_pos.y,
                        target_placement_pos.z,
                    ],
                )
            })
            .map(|state| state.block())
            .unwrap_or_else(|| hit_state.block());

        let right_click_delay = host.right_click_delay().unwrap_or(0);
        let current_mouse_ratio = compute_mouse_ratio(host.cursor(), host.window_size());

        let action = self.with_state(|st| {
            st.placement.evaluate_pick(
                hit_target,
                hit_pos,
                hit.face,
                target_placement_pos,
                old_target_block,
                player_pos,
                use_key_down,
                right_click_delay,
                current_mouse_ratio,
            )
        });

        match action {
            PickAction::AllowVanilla | PickAction::SuppressNoAction => {}
            PickAction::TriggerPlacement {
                override_target,
                reset_right_click_delay,
            } => {
                if let Some(target) = override_target {
                    *event.result = Some(target);
                }
                if reset_right_click_delay {
                    let _ = host.set_right_click_delay(0);
                }
                if let Some(key) = use_key {
                    self.with_state(|st| {
                        st.placement.allow_injected_use_click = true;
                    });
                    let _ = key.click(host);
                }
            }
        }
    }

    fn interaction_key_mapping_triggered(
        &self,
        _host: &mut dyn ClientHost,
        event: &mut InteractionKeyMappingTriggeredEvent,
    ) -> Flow {
        if event.key != InteractionKey::Use {
            return Flow::Continue;
        }

        let should_cancel = self.with_state(|st| {
            if !st.config.accurate_placement_enabled || !st.placement.disable_normal_item_use {
                return false;
            }
            if st.placement.allow_injected_use_click {
                // Izinkan satu rangkaian klik hasil injeksi ABP (Main/Off hand pada tick ini).
                if event.hand == Hand::Off {
                    st.placement.allow_injected_use_click = false;
                }
                return false;
            }
            true
        });

        if should_cancel {
            event.swing_hand = false;
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        // 1. Sinkronisasi awal setting Horizon <-> file lokal `config/accurateblockplacement.toml`.
        let (needs_sync, keys, loaded_local, local_cfg, config_dir) = self.with_state(|st| {
            (
                !st.settings_synced,
                st.setting_keys.clone(),
                st.loaded_local_config,
                st.config.clone(),
                st.config_dir.clone(),
            )
        });

        if needs_sync {
            if loaded_local {
                local_cfg.push_all_to_host(&keys, host);
            }
            let synced_cfg = self.with_state(|st| {
                st.config.sync_from_host(&st.setting_keys, &*host);
                st.settings_synced = true;
                st.config.clone()
            });
            if let Some(dir) = config_dir {
                synced_cfg.save_to_config_dir(&dir);
            }
        }

        // Bersihkan flag klik terinjeksi di akhir tick jika tangan off-hand tidak pernah dipanggil.
        self.with_state(|st| {
            st.placement.allow_injected_use_click = false;
        });

        // 2. Konfirmasi blok yang baru saja diletakkan pada tick ini.
        self.check_and_confirm_pending_placement(&*host);

        // 3. Tangani keybind Toggle Accurate Block Placement & Toggle Fast Block Breaking.
        let (toggle_place_key, toggle_break_key, attack_key) = self.with_state(|st| {
            (
                st.toggle_placement_key,
                st.toggle_fast_break_key,
                st.attack_key,
            )
        });

        if let Some(key) = toggle_place_key {
            while key.consume_click(host).unwrap_or(false) {
                let (new_val, setting_key, cfg_snapshot, dir_opt) = self.with_state(|st| {
                    st.config.accurate_placement_enabled = !st.config.accurate_placement_enabled;
                    if !st.config.accurate_placement_enabled {
                        st.placement.reset_all();
                    }
                    (
                        st.config.accurate_placement_enabled,
                        st.setting_keys.accurate_place,
                        st.config.clone(),
                        st.config_dir.clone(),
                    )
                });

                if let Some(sk) = setting_key {
                    let _ = host.set_setting(sk, SettingValue::Toggle(new_val));
                }
                if let Some(dir) = dir_opt {
                    cfg_snapshot.save_to_config_dir(&dir);
                }

                let msg_key = if new_val {
                    MSG_PLACEMENT_ENABLED
                } else {
                    MSG_PLACEMENT_DISABLED
                };
                Self::send_confirmation_message(host, &cfg_snapshot, msg_key);
            }
        }

        if let Some(key) = toggle_break_key {
            while key.consume_click(host).unwrap_or(false) {
                let (new_val, setting_key, cfg_snapshot, dir_opt) = self.with_state(|st| {
                    st.config.fast_breaking_enabled = !st.config.fast_breaking_enabled;
                    (
                        st.config.fast_breaking_enabled,
                        st.setting_keys.fast_break,
                        st.config.clone(),
                        st.config_dir.clone(),
                    )
                });

                if let Some(sk) = setting_key {
                    let _ = host.set_setting(sk, SettingValue::Toggle(new_val));
                }
                if let Some(dir) = dir_opt {
                    cfg_snapshot.save_to_config_dir(&dir);
                }

                let msg_key = if new_val {
                    MSG_FAST_BREAK_ENABLED
                } else {
                    MSG_FAST_BREAK_DISABLED
                };
                Self::send_confirmation_message(host, &cfg_snapshot, msg_key);
            }
        }

        // 4. Fast Block Breaking: saat tombol attack ditahan ke blok dan `mining_destroy_delay() > 0`,
        // picu klik attack segar agar tidak menunggu jeda 5 tick vanilla.
        let fast_break_on = self.with_state(|st| st.config.fast_breaking_enabled);
        if fast_break_on
            && host.playing()
            && host.mining_destroy_delay() > 0
            && matches!(host.crosshair_target(), Some(CrosshairTarget::Block { .. }))
            && let Some(atk) = attack_key
            && atk.physical_down(&*host).unwrap_or(false)
        {
            let _ = atk.click(host);
        }
    }

    fn client_level_unload(
        &self,
        _host: &mut dyn ClientHost,
        _event: &mut ClientLevelUnloadEvent<'_>,
    ) {
        self.with_state(|st| st.placement.reset_all());
    }

    fn client_player_logging_out(
        &self,
        _host: &mut dyn ClientHost,
        _event: &mut ClientPlayerLoggingOutEvent,
    ) {
        self.with_state(|st| st.placement.reset_all());
    }
}