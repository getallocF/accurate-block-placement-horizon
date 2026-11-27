use std::collections::VecDeque;

use mod_api::prelude::*;

pub const MAX_BACKFILL_QUEUE: usize = 64;
pub const BACKWARDS_BUILD_STEP_THRESHOLD: f64 = 0.99;
pub const MOUSE_MOVED_RATIO_THRESHOLD: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingPlacement {
    pub target_pos: BlockPos,
    pub old_block: BlockKey,
    pub face: [i32; 3],
    pub player_pos: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickAction {
    /// Biarkan vanilla menangani klik kanan secara normal.
    AllowVanilla,
    /// Tekan klik kanan vanilla; jangan lakukan penempatan pada pick ini.
    SuppressNoAction,
    /// Tekan klik kanan vanilla dan picu satu aksi penempatan pada target saat ini (atau backfill).
    TriggerPlacement {
        override_target: Option<CrosshairTarget>,
        reset_right_click_delay: bool,
    },
}

#[derive(Debug)]
pub struct PlacementState {
    pub disable_normal_item_use: bool,
    pub last_seen_block_pos: Option<BlockPos>,
    pub last_placed_block_pos: Option<BlockPos>,
    pub last_player_placed_block_pos: Option<[f64; 3]>,
    pub auto_repeat_waiting_on_cooldown: bool,
    pub last_fresh_press_mouse_ratio: Option<[f64; 2]>,
    pub backfill_queue: VecDeque<CrosshairTarget>,
    pub pending_placement: Option<PendingPlacement>,
    pub fresh_use_press_pending: bool,
    pub allow_injected_use_click: bool,
}

impl Default for PlacementState {
    fn default() -> Self {
        Self {
            disable_normal_item_use: false,
            last_seen_block_pos: None,
            last_placed_block_pos: None,
            last_player_placed_block_pos: None,
            auto_repeat_waiting_on_cooldown: true,
            last_fresh_press_mouse_ratio: None,
            backfill_queue: VecDeque::new(),
            pending_placement: None,
            fresh_use_press_pending: false,
            allow_injected_use_click: false,
        }
    }
}

impl PlacementState {
    /// Reset total saat mod dimatikan, keluar dunia, atau unload level.
    pub fn reset_all(&mut self) {
        self.disable_normal_item_use = false;
        self.last_seen_block_pos = None;
        self.last_placed_block_pos = None;
        self.last_player_placed_block_pos = None;
        self.auto_repeat_waiting_on_cooldown = true;
        self.last_fresh_press_mouse_ratio = None;
        self.backfill_queue.clear();
        self.pending_placement = None;
        self.fresh_use_press_pending = false;
        self.allow_injected_use_click = false;
    }

    /// Dipanggil saat tombol `key.use` ditekan secara fisik (fresh press).
    pub fn mark_fresh_use_press(&mut self, mouse_ratio: Option<[f64; 2]>) {
        self.fresh_use_press_pending = true;
        self.last_seen_block_pos = None;
        self.last_placed_block_pos = None;
        self.last_player_placed_block_pos = None;
        self.auto_repeat_waiting_on_cooldown = true;
        self.backfill_queue.clear();
        self.last_fresh_press_mouse_ratio = mouse_ratio;
    }

    /// Dipanggil saat tombol `key.use` dilepas secara fisik.
    pub fn mark_use_released(&mut self) {
        self.disable_normal_item_use = false;
        self.last_seen_block_pos = None;
        self.last_placed_block_pos = None;
        self.last_player_placed_block_pos = None;
        self.auto_repeat_waiting_on_cooldown = true;
        self.last_fresh_press_mouse_ratio = None;
        self.backfill_queue.clear();
        self.fresh_use_press_pending = false;
    }

    /// Mengonfirmasi apakah blok di `target_pos` benar-benar berubah setelah aksi penempatan,
    /// lalu memperbarui `last_placed_block_pos` dan `last_player_placed_block_pos` secara anti-drift.
    pub fn confirm_pending_placement(&mut self, current_block_at_target: Option<BlockKey>) {
        let Some(pending) = self.pending_placement.take() else {
            return;
        };
        let Some(now_block) = current_block_at_target else {
            return;
        };
        if now_block != pending.old_block {
            self.last_placed_block_pos = Some(pending.target_pos);
            self.last_player_placed_block_pos = Some(advance_player_placed_pos(
                self.last_player_placed_block_pos,
                pending.player_pos,
                pending.face,
            ));
        }
    }

    /// Evaluasi inti Accurate Block Placement pada setiap fase `client_pick`,
    /// mereplikasi 1:1 logika `MinecraftMixin.onUpdateTargetedEntityComplete`.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_pick(
        &mut self,
        hit_target: CrosshairTarget,
        hit_pos: BlockPos,
        clicked_face: [i32; 3],
        target_placement_pos: BlockPos,
        old_target_block: BlockKey,
        player_pos: [f64; 3],
        use_key_down: bool,
        right_click_delay: u32,
        current_mouse_ratio: Option<[f64; 2]>,
    ) -> PickAction {
        let fresh_key_press = self.fresh_use_press_pending;
        self.fresh_use_press_pending = false;

        if fresh_key_press {
            self.last_seen_block_pos = None;
            self.last_placed_block_pos = None;
            self.last_player_placed_block_pos = None;
            self.auto_repeat_waiting_on_cooldown = true;
            self.backfill_queue.clear();
            if current_mouse_ratio.is_some() {
                self.last_fresh_press_mouse_ratio = current_mouse_ratio;
            }
        }

        // Selama item valid dan crosshair mengenai blok, ambil alih timing item-use vanilla.
        self.disable_normal_item_use = true;

        if !use_key_down && !fresh_key_press {
            self.last_seen_block_pos = None;
            self.last_placed_block_pos = None;
            self.last_player_placed_block_pos = None;
            self.auto_repeat_waiting_on_cooldown = true;
            self.backfill_queue.clear();
            return PickAction::SuppressNoAction;
        }

        // Jika ada target backfill yang menunggu setelah cooldown selesai, putar ulang satu per satu.
        if !self.auto_repeat_waiting_on_cooldown
            && !fresh_key_press
            && let Some(queued_target) = self.backfill_queue.pop_front()
        {
            if let CrosshairTarget::Block { pos, state, hit } = queued_target {
                let queued_place_pos = offset_pos(pos, hit.face);
                self.pending_placement = Some(PendingPlacement {
                    target_pos: queued_place_pos,
                    old_block: state.block(),
                    face: hit.face,
                    player_pos,
                });
            }
            return PickAction::TriggerPlacement {
                override_target: Some(queued_target),
                reset_right_click_delay: true,
            };
        }

        let target_fresh = is_placement_target_fresh(
            hit_pos,
            clicked_face,
            player_pos,
            self.last_seen_block_pos,
            self.last_placed_block_pos,
            self.last_player_placed_block_pos,
        );

        let mouse_moved = has_mouse_moved(current_mouse_ratio, self.last_fresh_press_mouse_ratio);
        let on_cooldown =
            self.auto_repeat_waiting_on_cooldown && right_click_delay > 0 && !mouse_moved;

        let action = if fresh_key_press || (target_fresh && !on_cooldown) {
            if self.auto_repeat_waiting_on_cooldown && !fresh_key_press {
                self.auto_repeat_waiting_on_cooldown = false;
                // Jika backfill terisi selama cooldown, antrekan target sekarang di belakang
                // dan jalankan item backfill pertama lebih dahulu.
                if let Some(first_backfill) = self.backfill_queue.pop_front() {
                    if self.backfill_queue.len() < MAX_BACKFILL_QUEUE {
                        self.backfill_queue.push_back(hit_target);
                    }
                    if let CrosshairTarget::Block { pos, state, hit } = first_backfill {
                        self.pending_placement = Some(PendingPlacement {
                            target_pos: offset_pos(pos, hit.face),
                            old_block: state.block(),
                            face: hit.face,
                            player_pos,
                        });
                    }
                    self.last_seen_block_pos = Some(hit_pos);
                    return PickAction::TriggerPlacement {
                        override_target: Some(first_backfill),
                        reset_right_click_delay: true,
                    };
                }
            }

            self.pending_placement = Some(PendingPlacement {
                target_pos: target_placement_pos,
                old_block: old_target_block,
                face: clicked_face,
                player_pos,
            });

            PickAction::TriggerPlacement {
                override_target: None,
                reset_right_click_delay: !fresh_key_press,
            }
        } else if target_fresh {
            if self.backfill_queue.len() < MAX_BACKFILL_QUEUE {
                self.backfill_queue.push_back(hit_target);
            }
            PickAction::SuppressNoAction
        } else {
            PickAction::SuppressNoAction
        };

        self.last_seen_block_pos = Some(hit_pos);
        action
    }
}

#[inline]
pub fn offset_pos(pos: BlockPos, face: [i32; 3]) -> BlockPos {
    BlockPos {
        x: pos.x.saturating_add(face[0]),
        y: pos.y.saturating_add(face[1]),
        z: pos.z.saturating_add(face[2]),
    }
}

#[inline]
pub fn axis_index_from_face(face: [i32; 3]) -> usize {
    if face[0] != 0 {
        0
    } else if face[1] != 0 {
        1
    } else {
        2
    }
}

/// Menghitung koordinat tepi blok pada sumbu `clicked_face`, persis seperti rumus Java:
/// ditambah `+1.0` untuk face West (`[-1, 0, 0]`) dan North (`[0, 0, -1]`).
#[inline]
pub fn facing_axis_block_edge(last_placed: BlockPos, face: [i32; 3], axis: usize) -> f64 {
    let base = match axis {
        0 => f64::from(last_placed.x),
        1 => f64::from(last_placed.y),
        _ => f64::from(last_placed.z),
    };
    let edge_adjust = if face == [-1, 0, 0] || face == [0, 0, -1] {
        1.0
    } else {
        0.0
    };
    base + edge_adjust
}

/// Rumus asli `isPlacementTargetFresh` dari `MinecraftMixin.java`.
pub fn is_placement_target_fresh(
    hit_pos: BlockPos,
    clicked_face: [i32; 3],
    player_pos: [f64; 3],
    last_seen_block_pos: Option<BlockPos>,
    last_placed_block_pos: Option<BlockPos>,
    last_player_placed_block_pos: Option<[f64; 3]>,
) -> bool {
    if last_seen_block_pos != Some(hit_pos) && last_placed_block_pos != Some(hit_pos) {
        return true;
    }

    if last_placed_block_pos == Some(hit_pos)
        && let (Some(last_placed), Some(last_player_pos)) =
            (last_placed_block_pos, last_player_placed_block_pos)
    {
        let axis = axis_index_from_face(clicked_face);
        let facing_now = player_pos[axis];
        let facing_last = last_player_pos[axis];
        let facing_block = facing_axis_block_edge(last_placed, clicked_face, axis);

        let moved_enough = (facing_last - facing_now).abs() >= BACKWARDS_BUILD_STEP_THRESHOLD;
        let moving_away = (facing_last - facing_block).abs() < (facing_now - facing_block).abs();
        return moved_enough && moving_away;
    }

    false
}

/// Rumus asli deteksi perpindahan kursor mouse selama jendela cooldown:
/// jarak Euclidean rasio layar `>= 0.1`.
pub fn has_mouse_moved(
    current_ratio: Option<[f64; 2]>,
    fresh_press_ratio: Option<[f64; 2]>,
) -> bool {
    let (Some(cur), Some(fresh)) = (current_ratio, fresh_press_ratio) else {
        return false;
    };
    let dx = cur[0] - fresh[0];
    let dy = cur[1] - fresh[1];
    dx.hypot(dy) >= MOUSE_MOVED_RATIO_THRESHOLD
}

/// Rumus anti-drift `lastPlayerPlacedBlockPos` dari `MinecraftMixin.java`:
/// Pada penempatan pertama menyimpan `current`; pada penempatan lanjutan hanya memajukan
/// sumbu face sebanyak 1 blok dari nilai sebelumnya sambil memakai koordinat aktual pemain
/// pada dua sumbu lainnya.
pub fn advance_player_placed_pos(
    prev: Option<[f64; 3]>,
    current: [f64; 3],
    face: [i32; 3],
) -> [f64; 3] {
    let Some(last) = prev else {
        return current;
    };

    let axis = axis_index_from_face(face);
    let mut next = current;
    next[axis] = last[axis] + f64::from(face[axis]);
    next
}

pub fn compute_mouse_ratio(cursor: [f32; 2], window_size: Option<[u32; 2]>) -> Option<[f64; 2]> {
    let [w, h] = window_size?;
    if w == 0 || h == 0 {
        return None;
    }
    Some([
        f64::from(cursor[0]) / f64::from(w),
        f64::from(cursor[1]) / f64::from(h),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backwards_building_triggers_only_when_moving_away_by_at_least_0_99() {
        let block = BlockPos { x: 10, y: 64, z: 0 };
        let west_face = [-1, 0, 0];
        let last_player = [10.0, 64.0, 0.5];

        // Gerak < 0.99 belum dianggap target baru
        assert!(!is_placement_target_fresh(
            block,
            west_face,
            [9.05, 64.0, 0.5],
            Some(block),
            Some(block),
            Some(last_player),
        ));

        // Gerak >= 0.99 menjauhi sisi West (tepi blok = 10 + 1 = 11.0)
        assert!(is_placement_target_fresh(
            block,
            west_face,
            [9.0, 64.0, 0.5],
            Some(block),
            Some(block),
            Some(last_player),
        ));

        // Gerak mendekati blok tidak memicu backwards building
        assert!(!is_placement_target_fresh(
            block,
            west_face,
            [11.0, 64.0, 0.5],
            Some(block),
            Some(block),
            Some(last_player),
        ));
    }

    #[test]
    fn anti_drift_advances_only_face_axis_from_previous() {
        let first = advance_player_placed_pos(None, [10.25, 64.0, 5.5], [1, 0, 0]);
        assert_eq!(first, [10.25, 64.0, 5.5]);

        let second = advance_player_placed_pos(Some(first), [11.40, 64.2, 5.9], [1, 0, 0]);
        assert_eq!(second, [11.25, 64.2, 5.9]);
    }

    #[test]
    fn mouse_ratio_threshold_matches_euclidean_0_1() {
        assert!(!has_mouse_moved(Some([0.55, 0.55]), Some([0.50, 0.50])));
        assert!(has_mouse_moved(Some([0.60, 0.50]), Some([0.50, 0.50])));
    }
}