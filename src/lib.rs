use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use horizon_mod_api::prelude::*;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "config/accurate_block_placement.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
enum BlockDirection {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}

impl BlockDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Down => "DOWN",
            Self::Up => "UP",
            Self::North => "NORTH",
            Self::South => "SOUTH",
            Self::West => "WEST",
            Self::East => "EAST",
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    fn from_name(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "down" | "bawah" | "0" => Some(Self::Down),
            "up" | "atas" | "1" => Some(Self::Up),
            "north" | "utara" | "2" => Some(Self::North),
            "south" | "selatan" | "3" => Some(Self::South),
            "west" | "barat" | "4" => Some(Self::West),
            "east" | "timur" | "5" => Some(Self::East),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct AbpConfig {
    enabled: bool,
    fast_place: bool,
    prevent_double_place: bool,
    carpet_protocol: bool,
    reverse_facing: bool,
}

impl Default for AbpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fast_place: true,
            prevent_double_place: true,
            carpet_protocol: true,
            reverse_facing: false,
        }
    }
}

impl AbpConfig {
    fn load() -> Self {
        let path = PathBuf::from(CONFIG_PATH);
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<Self>(&raw) {
                return parsed;
            }
        }
        let default_cfg = Self::default();
        default_cfg.save();
        default_cfg
    }

    fn save(&self) {
        let path = PathBuf::from(CONFIG_PATH);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(serialized) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, serialized);
        }
    }
}

struct ModState {
    config: AbpConfig,
    locked_direction: Option<BlockDirection>,
    last_placed_pos: Option<[i32; 3]>,
    last_placed_tick: u64,
    current_tick: u64,
}

impl ModState {
    fn new() -> Self {
        Self {
            config: AbpConfig::load(),
            locked_direction: None,
            last_placed_pos: None,
            last_placed_tick: 0,
            current_tick: 0,
        }
    }
}

static STATE: OnceLock<Mutex<ModState>> = OnceLock::new();

fn state() -> &'static Mutex<ModState> {
    STATE.get_or_init(|| Mutex::new(ModState::new()))
}

/// Menghitung arah hadap blok berdasarkan sudut yaw & pitch kamera pemain
fn direction_from_look(yaw: f32, pitch: f32) -> BlockDirection {
    if pitch < -45.0 {
        return BlockDirection::Up;
    }
    if pitch > 45.0 {
        return BlockDirection::Down;
    }
    let normalized = yaw.rem_euclid(360.0);
    if !(45.0..315.0).contains(&normalized) {
        BlockDirection::South
    } else if normalized < 135.0 {
        BlockDirection::West
    } else if normalized < 225.0 {
        BlockDirection::North
    } else {
        BlockDirection::East
    }
}

fn toggle_mod(client: &mut ClientContext) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.config.enabled = !st.config.enabled;
    st.last_placed_pos = None;
    st.config.save();

    let msg = if st.config.enabled {
        "§aAccurate Block Placement: ON"
    } else {
        "§cAccurate Block Placement: OFF"
    };
    client.send_actionbar(msg);
}

fn toggle_fast_place(client: &mut ClientContext) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.config.fast_place = !st.config.fast_place;
    st.config.save();

    let msg = if st.config.fast_place {
        "§aFast Place (0 Cooldown): ON"
    } else {
        "§cFast Place (0 Cooldown): OFF"
    };
    client.send_actionbar(msg);
}

fn toggle_reverse(client: &mut ClientContext) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.config.reverse_facing = !st.config.reverse_facing;
    st.config.save();

    let msg = if st.config.reverse_facing {
        "§dReverse Block Facing: ON"
    } else {
        "§7Reverse Block Facing: OFF"
    };
    client.send_actionbar(msg);
}

fn toggle_or_set_lock(client: &mut ClientContext, explicit: Option<Option<BlockDirection>>) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    match explicit {
        Some(dir_opt) => {
            st.locked_direction = dir_opt;
        }
        None => {
            if st.locked_direction.is_some() {
                st.locked_direction = None;
            } else {
                let dir = direction_from_look(client.player_yaw(), client.player_pitch());
                st.locked_direction = Some(dir);
            }
        }
    }

    match st.locked_direction {
        Some(dir) => client.send_actionbar(&format!("§bRotation Lock: {}", dir.as_str())),
        None => client.send_actionbar("§eRotation Lock: OFF"),
    }
}

#[horizon_mod_main]
pub fn init(ctx: &mut ModContext) {
    // 1. Keybind Desktop / External Keyboard
    ctx.register_keybind("key.accurate_block_placement.toggle", KeyCode::KeyR, |client| {
        toggle_mod(client);
    });

    ctx.register_keybind("key.accurate_block_placement.lock", KeyCode::KeyG, |client| {
        toggle_or_set_lock(client, None);
    });

    ctx.register_keybind("key.accurate_block_placement.reverse", KeyCode::KeyV, |client| {
        toggle_reverse(client);
    });

    ctx.register_keybind("key.accurate_block_placement.fastplace", KeyCode::KeyB, |client| {
        toggle_fast_place(client);
    });

    // 2. Client Command /abp (Ramah Layar Sentuh Horizon Android)
    ctx.register_client_command("abp", |client, args| {
        let sub = args.first().map(|s| s.to_ascii_lowercase());
        match sub.as_deref() {
            None | Some("toggle") => toggle_mod(client),
            Some("fast") | Some("fastplace") => toggle_fast_place(client),
            Some("reverse") | Some("rev") => toggle_reverse(client),
            Some("lock") => {
                if let Some(arg_dir) = args.get(1) {
                    if arg_dir.eq_ignore_ascii_case("off") || arg_dir.eq_ignore_ascii_case("unlock") {
                        toggle_or_set_lock(client, Some(None));
                    } else if let Some(parsed_dir) = BlockDirection::from_name(arg_dir) {
                        toggle_or_set_lock(client, Some(Some(parsed_dir)));
                    } else {
                        client.send_chat("§cArah tidak valid. Gunakan: north, south, east, west, up, down, atau off");
                    }
                } else {
                    toggle_or_set_lock(client, None);
                }
            }
            Some("status") | Some("info") => {
                let st = state().lock().unwrap_or_else(|e| e.into_inner());
                let lock_str = st
                    .locked_direction
                    .map(|d| d.as_str())
                    .unwrap_or("OFF");
                client.send_chat(&format!(
                    "§6[ABP] §fMod: {} §7| §fFastPlace: {} §7| §fReverse: {} §7| §fLock: §b{}",
                    if st.config.enabled { "§aON" } else { "§cOFF" },
                    if st.config.fast_place { "§aON" } else { "§cOFF" },
                    if st.config.reverse_facing { "§dON" } else { "§7OFF" },
                    lock_str
                ));
            }
            _ => {
                client.send_chat("§ePenggunaan: /abp <toggle|fast|lock [arah|off]|reverse|status>");
            }
        }
    });

    // 3. Client Tick: Hilangkan cooldown klik kanan saat memegang blok (Fast Place)
    ctx.on_client_tick(|client| {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.current_tick = st.current_tick.wrapping_add(1);

        if !st.config.enabled {
            return;
        }

        if st.config.fast_place && client.is_holding_block() && client.item_use_cooldown() > 0 {
            client.set_item_use_cooldown(0);
        }

        // Reset riwayat posisi saat tombol klik kanan / pasang blok dilepas
        if !client.is_use_key_pressed() {
            st.last_placed_pos = None;
        }
    });

    // 4. Intercept Block Placement: Anti-Double-Place & Carpet Accurate Block Placement Protocol
    ctx.on_block_place(|event| {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if !st.config.enabled {
            return;
        }

        let target_pos = event.target_block_pos();

        // Cegah blok bertumpuk 2x di titik identik dalam rentang 2 tick saat menahan tombol pasang
        if st.config.prevent_double_place
            && st.last_placed_pos == Some(target_pos)
            && st.current_tick.saturating_sub(st.last_placed_tick) <= 2
        {
            event.cancel();
            return;
        }

        st.last_placed_pos = Some(target_pos);
        st.last_placed_tick = st.current_tick;

        if !st.config.carpet_protocol {
            return;
        }

        // Tentukan arah blok (prioritas: Rotation Lock -> Reverse Facing)
        let resolved_dir = match st.locked_direction {
            Some(locked) => Some(if st.config.reverse_facing {
                locked.opposite()
            } else {
                locked
            }),
            None if st.config.reverse_facing => {
                Some(direction_from_look(event.player_yaw(), event.player_pitch()).opposite())
            }
            None => None,
        };

        if let Some(dir) = resolved_dir {
            // Encoding Carpet Protocol pada koordinat hit_x: block_x + 2.0 + (dir_id * 2.0)
            let carpet_hit_x = (target_pos[0] as f64) + 2.0 + ((dir as u8 as f64) * 2.0);
            event.set_hit_x(carpet_hit_x);
        }
    });
}
