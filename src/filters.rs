use mod_api::prelude::*;

use crate::config::AbpConfig;

#[inline]
pub fn strip_minecraft_ns(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// Mereplikasi `MinecraftMixin.isItemAllowed` dari Accurate Block Placement Reborn:
/// - Semua `BlockItem` (vanilla maupun modded melalui `host.item_block`)
/// - Tools (`DataComponents.BLOCK_TRANSFORMER`: axe, shovel, hoe + `flint_and_steel`)
/// - Buckets (kecuali `milk_bucket`)
/// - Armor Stands
/// - Item Frames & Glow Item Frames
/// - Spawn Eggs (`*_spawn_egg`)
pub fn is_item_allowed(host: &dyn ClientHost, item: ItemKey, cfg: &AbpConfig) -> bool {
    if host.item_block(item).is_some() {
        return true;
    }

    let Some(raw_id) = host.item_key_id(item) else {
        return false;
    };
    let id = strip_minecraft_ns(raw_id);

    if cfg.tools_enabled && is_block_transformer_or_flint_tool(id) {
        return true;
    }
    if cfg.bucket_enabled && is_placeable_bucket(id) {
        return true;
    }
    if cfg.armor_stand_enabled && id == "armor_stand" {
        return true;
    }
    if cfg.item_frame_enabled && matches!(id, "item_frame" | "glow_item_frame") {
        return true;
    }
    if cfg.spawn_eggs_enabled && id.ends_with("_spawn_egg") {
        return true;
    }

    false
}

pub fn is_block_transformer_or_flint_tool(id: &str) -> bool {
    id == "flint_and_steel"
        || id.ends_with("_axe")
        || id.ends_with("_shovel")
        || id.ends_with("_hoe")
}

pub fn is_placeable_bucket(id: &str) -> bool {
    matches!(
        id,
        "bucket"
            | "water_bucket"
            | "lava_bucket"
            | "powder_snow_bucket"
            | "cod_bucket"
            | "salmon_bucket"
            | "pufferfish_bucket"
            | "tropical_fish_bucket"
            | "axolotl_bucket"
            | "tadpole_bucket"
    )
}

/// Mereplikasi `doesItemHaveOverriddenUseMethod` untuk item yang punya aksi klik-kanan khusus
/// (makanan, ramuan, senjata proyektil, perisai, buku, dll.).
/// Item yang diizinkan oleh `is_item_allowed` diperiksa terlebih dahulu oleh pemanggil.
pub fn does_item_have_overridden_use(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);

    if id.ends_with("_boat")
        || id.ends_with("_chest_boat")
        || id.ends_with("_raft")
        || id.ends_with("_chest_raft")
        || id.ends_with("_helmet")
        || id.ends_with("_chestplate")
        || id.ends_with("_leggings")
        || id.ends_with("_boots")
        || id.ends_with("_bundle")
        || id.ends_with("_egg")
    {
        return true;
    }

    matches!(
        id,
        "bow"
            | "crossbow"
            | "trident"
            | "shield"
            | "spyglass"
            | "goat_horn"
            | "fishing_rod"
            | "carrot_on_a_stick"
            | "warped_fungus_on_a_stick"
            | "ender_pearl"
            | "ender_eye"
            | "snowball"
            | "wind_charge"
            | "firework_rocket"
            | "experience_bottle"
            | "potion"
            | "splash_potion"
            | "lingering_potion"
            | "ominous_bottle"
            | "milk_bucket"
            | "honey_bottle"
            | "writable_book"
            | "written_book"
            | "map"
            | "filled_map"
            | "bundle"
            | "elytra"
            | "brush"
            | "apple"
            | "golden_apple"
            | "enchanted_golden_apple"
            | "melon_slice"
            | "sweet_berries"
            | "glow_berries"
            | "chorus_fruit"
            | "carrot"
            | "golden_carrot"
            | "potato"
            | "baked_potato"
            | "poisonous_potato"
            | "beetroot"
            | "dried_kelp"
            | "beef"
            | "cooked_beef"
            | "porkchop"
            | "cooked_porkchop"
            | "mutton"
            | "cooked_mutton"
            | "chicken"
            | "cooked_chicken"
            | "rabbit"
            | "cooked_rabbit"
            | "cod"
            | "cooked_cod"
            | "salmon"
            | "cooked_salmon"
            | "tropical_fish"
            | "pufferfish"
            | "bread"
            | "cookie"
            | "pumpkin_pie"
            | "rotten_flesh"
            | "spider_eye"
            | "mushroom_stew"
            | "beetroot_soup"
            | "rabbit_stew"
            | "suspicious_stew"
    )
}

pub fn is_stair_block(raw_id: &str) -> bool {
    strip_minecraft_ns(raw_id).ends_with("_stairs")
}

/// Mereplikasi pemeriksaan `targetBlock instanceof BaseEntityBlock || targetBlock instanceof BedBlock`
/// agar pemain tidak membuka container / kasur berulang kali saat menahan klik kanan.
pub fn is_storage_or_bed_block(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);

    if id.ends_with("_bed")
        || id.ends_with("_shulker_box")
        || id.ends_with("_sign")
        || id.ends_with("_hanging_sign")
        || id.ends_with("_banner")
        || id.ends_with("_skull")
        || id.ends_with("_head")
        || id.ends_with("_copper_chest")
    {
        return true;
    }

    matches!(
        id,
        "chest"
            | "trapped_chest"
            | "ender_chest"
            | "barrel"
            | "shulker_box"
            | "furnace"
            | "blast_furnace"
            | "smoker"
            | "hopper"
            | "dropper"
            | "dispenser"
            | "crafter"
            | "brewing_stand"
            | "beacon"
            | "enchanting_table"
            | "lectern"
            | "jukebox"
            | "campfire"
            | "soul_campfire"
            | "beehive"
            | "bee_nest"
            | "chiseled_bookshelf"
            | "decorated_pot"
            | "command_block"
            | "chain_command_block"
            | "repeating_command_block"
            | "structure_block"
            | "jigsaw"
            | "spawner"
            | "trial_spawner"
            | "vault"
            | "conduit"
            | "sculk_sensor"
            | "calibrated_sculk_sensor"
            | "sculk_shrieker"
            | "sculk_catalyst"
            | "end_gateway"
            | "end_portal"
            | "daylight_detector"
            | "comparator"
            | "bell"
            | "suspicious_sand"
            | "suspicious_gravel"
            | "creaking_heart"
            | "test_block"
            | "test_instance_block"
    )
}

/// Mereplikasi `doesBlockHaveOverriddenActivateMethod` untuk blok yang punya interaksi klik-kanan.
pub fn is_activatable_block(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);

    if is_storage_or_bed_block(id) {
        return true;
    }

    if id.ends_with("_door")
        || id.ends_with("_trapdoor")
        || id.ends_with("_fence_gate")
        || id.ends_with("_button")
        || id.ends_with("_candle_cake")
        || id.ends_with("_candle")
        || id.ends_with("_bulb")
        || id.starts_with("potted_")
    {
        return true;
    }

    matches!(
        id,
        "lever"
            | "repeater"
            | "note_block"
            | "crafting_table"
            | "loom"
            | "cartography_table"
            | "fletching_table"
            | "smithing_table"
            | "grindstone"
            | "stonecutter"
            | "anvil"
            | "chipped_anvil"
            | "damaged_anvil"
            | "cake"
            | "candle"
            | "candle_cake"
            | "dragon_egg"
            | "flower_pot"
            | "sweet_berry_bush"
            | "cave_vines"
            | "cave_vines_plant"
            | "respawn_anchor"
            | "cauldron"
            | "water_cauldron"
            | "lava_cauldron"
            | "powder_snow_cauldron"
            | "redstone_wire"
            | "redstone_ore"
            | "deepslate_redstone_ore"
            | "tnt"
            | "composter"
            | "copper_bulb"
            | "exposed_copper_bulb"
            | "weathered_copper_bulb"
            | "oxidized_copper_bulb"
            | "waxed_copper_bulb"
            | "waxed_exposed_copper_bulb"
            | "waxed_weathered_copper_bulb"
            | "waxed_oxidized_copper_bulb"
    )
}

pub fn is_scaffolding(raw_id: &str) -> bool {
    strip_minecraft_ns(raw_id) == "scaffolding"
}

pub fn is_composter_block(raw_id: &str) -> bool {
    strip_minecraft_ns(raw_id) == "composter"
}

pub fn is_compostable_item(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);

    if id.ends_with("_leaves")
        || id.ends_with("_sapling")
        || id.ends_with("_seeds")
        || id.ends_with("_roots")
        || id.ends_with("_fungus")
        || id.ends_with("_tulip")
        || id.ends_with("_petals")
    {
        return true;
    }

    matches!(
        id,
        "beetroot"
            | "dried_kelp"
            | "short_grass"
            | "tall_grass"
            | "fern"
            | "large_fern"
            | "kelp"
            | "seagrass"
            | "sweet_berries"
            | "glow_berries"
            | "hanging_roots"
            | "small_dripleaf"
            | "big_dripleaf"
            | "moss_carpet"
            | "moss_block"
            | "pale_moss_carpet"
            | "pale_moss_block"
            | "pale_hanging_moss"
            | "vine"
            | "glow_lichen"
            | "twisting_vines"
            | "weeping_vines"
            | "cactus"
            | "cactus_flower"
            | "sugar_cane"
            | "dried_kelp_block"
            | "melon_slice"
            | "melon"
            | "pumpkin"
            | "carved_pumpkin"
            | "apple"
            | "azalea"
            | "flowering_azalea"
            | "brown_mushroom"
            | "red_mushroom"
            | "brown_mushroom_block"
            | "red_mushroom_block"
            | "mushroom_stem"
            | "cocoa_beans"
            | "lily_pad"
            | "nether_wart"
            | "nether_wart_block"
            | "warped_wart_block"
            | "potato"
            | "baked_potato"
            | "carrot"
            | "wheat"
            | "hay_block"
            | "bread"
            | "cookie"
            | "cake"
            | "pumpkin_pie"
            | "shroomlight"
            | "spore_blossom"
            | "sea_pickle"
            | "dandelion"
            | "poppy"
            | "blue_orchid"
            | "allium"
            | "azure_bluet"
            | "oxeye_daisy"
            | "cornflower"
            | "lily_of_the_valley"
            | "wither_rose"
            | "torchflower"
            | "pitcher_plant"
            | "pitcher_pod"
            | "sunflower"
            | "lilac"
            | "rose_bush"
            | "peony"
            | "open_eyeblossom"
            | "closed_eyeblossom"
            | "wildflowers"
            | "leaf_litter"
            | "bush"
            | "firefly_bush"
            | "short_dry_grass"
            | "tall_dry_grass"
            | "nether_sprouts"
    )
}

pub fn is_bone_meal_item(raw_id: &str) -> bool {
    strip_minecraft_ns(raw_id) == "bone_meal"
}

pub fn is_bonemealable_block(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);

    if id.ends_with("_sapling")
        || id.ends_with("_fungus")
        || id.ends_with("_nylium")
        || id.ends_with("_stem")
        || id.ends_with("_tulip")
        || id.ends_with("_propagule")
    {
        return true;
    }

    matches!(
        id,
        "grass_block"
            | "moss_block"
            | "pale_moss_block"
            | "rooted_dirt"
            | "wheat"
            | "carrots"
            | "potatoes"
            | "beetroots"
            | "torchflower_crop"
            | "pitcher_crop"
            | "cocoa"
            | "sweet_berry_bush"
            | "cave_vines"
            | "cave_vines_plant"
            | "bamboo"
            | "bamboo_sapling"
            | "kelp"
            | "kelp_plant"
            | "seagrass"
            | "sea_pickle"
            | "short_grass"
            | "fern"
            | "brown_mushroom"
            | "red_mushroom"
            | "azalea"
            | "flowering_azalea"
            | "big_dripleaf"
            | "big_dripleaf_stem"
            | "small_dripleaf"
            | "twisting_vines"
            | "twisting_vines_plant"
            | "weeping_vines"
            | "weeping_vines_plant"
            | "glow_lichen"
            | "pink_petals"
            | "wildflowers"
            | "sunflower"
            | "lilac"
            | "rose_bush"
            | "peony"
            | "bush"
            | "firefly_bush"
            | "short_dry_grass"
            | "tall_dry_grass"
            | "sniffer_egg"
    )
}

/// Mereplikasi `BlockState.canBeReplaced(BlockPlaceContext)` untuk blok-blok vanilla umum
/// agar penentuan `targetPlacementPos` akurat.
pub fn is_replaceable_block(raw_id: &str) -> bool {
    let id = strip_minecraft_ns(raw_id);
    matches!(
        id,
        "air"
            | "cave_air"
            | "void_air"
            | "water"
            | "lava"
            | "short_grass"
            | "tall_grass"
            | "fern"
            | "large_fern"
            | "dead_bush"
            | "seagrass"
            | "tall_seagrass"
            | "snow"
            | "vine"
            | "glow_lichen"
            | "sculk_vein"
            | "hanging_roots"
            | "fire"
            | "soul_fire"
            | "structure_void"
            | "light"
            | "bubble_column"
            | "crimson_roots"
            | "warped_roots"
            | "nether_sprouts"
            | "bush"
            | "firefly_bush"
            | "short_dry_grass"
            | "tall_dry_grass"
            | "leaf_litter"
            | "wildflowers"
            | "pink_petals"
    )
}