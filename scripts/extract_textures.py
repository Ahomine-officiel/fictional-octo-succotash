#!/usr/bin/env python3
"""
Extract real textures from the two JARs into assets/textures/minecraft/
(so Assets::load_candidate finds them) + assets/skins/.

Sources:
  dl/x_dungeons/assets/minecraft/textures/block/  -> MCD-styled block overrides
  dl/x_dungeons/assets/duneons/textures/item/     -> real MCD weapons/artifacts
  dl/x_dungeons/assets/duneons/textures/entities/ -> MCD mobs (emerald, heart...)
  dl/x_client/assets/minecraft/textures/          -> vanilla gaps (stone, logs, items)
  dl/x_client/.../entity/player/{wide,slim}/      -> REAL Steve / Alex skins
  dl/x_client/.../entity/chest/normal.png         -> chest face crops

Destination layout (matching assets.rs candidates):
  assets/textures/minecraft/block/<vanilla_name>.png
  assets/textures/minecraft/item/<vanilla_name>.png
  assets/textures/minecraft/<key>.png            (custom keys: icon_*, art_*)
"""
from PIL import Image
import os, shutil, glob

BASE = "/home/z/my-project/mdungeons"
MOD = f"{BASE}/assets/dl/x_dungeons/assets"
CLI = f"{BASE}/assets/dl/x_client/assets"
DST = f"{BASE}/assets/textures/minecraft"
SKINS = f"{BASE}/assets/skins"
os.makedirs(f"{DST}/block", exist_ok=True)
os.makedirs(f"{DST}/item", exist_ok=True)
os.makedirs(SKINS, exist_ok=True)

stats = {"mod_block": 0, "cli_block": 0, "item": 0, "misc": 0}

# ---------------------------------------------------------------- blocks ----
# 1. ALL MCD-styled block overrides from the dungeons mod (skip overlays/mcmeta)
skip_suffix = ("_outer.png", ".mcmeta")
for src in glob.glob(f"{MOD}/minecraft/textures/block/*.png"):
    name = os.path.basename(src)
    if name.endswith(skip_suffix) or name == "desktop.ini":
        continue
    shutil.copy(src, f"{DST}/block/{name}")
    stats["mod_block"] += 1

# 2. Vanilla gaps the mod doesn't cover
CLI_BLOCK = f"{CLI}/minecraft/textures/block"
gap_blocks = ["stone.png", "gravel.png", "clay.png", "red_sand.png", "mud.png",
              "sculk.png", "deepslate.png", "obsidian.png", "purpur_block.png",
              "end_stone.png", "redstone_ore.png", "glowstone.png", "spawner.png",
              "anvil.png", "oak_log.png", "jungle_log.png", "pumpkin_top.png",
              "pumpkin_side.png", "carved_pumpkin.png", "lava_still.png",
              "moss_block.png", "end_portal.png", "redstone_block.png",
              "dark_oak_log.png", "spruce_log.png", "birch_log.png",
              "grass_block_side_overlay.png"]
for name in gap_blocks:
    src = f"{CLI_BLOCK}/{name}"
    if os.path.exists(src):
        shutil.copy(src, f"{DST}/block/{name}")
        stats["cli_block"] += 1

# ------------------------------------------------------------- chest crops --
chest = Image.open(f"{CLI}/minecraft/textures/entity/chest/normal.png").convert("RGBA")

def chest_face(lid_box, bot_box):
    """Compose lid row strip over body strip into a 16x16 face."""
    lx, ly = lid_box
    bx, by = bot_box
    top = chest.crop((lx, ly, lx + 14, ly + 5))    # lid front 14x5
    bot = chest.crop((bx, by, bx + 14, by + 10))   # body front 14x10
    out = Image.new("RGBA", (14, 15), (0, 0, 0, 0))
    out.paste(top, (0, 0))
    out.paste(bot, (0, 5))
    return out.resize((16, 16), Image.NEAREST)

# front: lid front (14,14) + body front (14,33) + latch (0,0,2,4)
front = chest_face((14, 14), (14, 33))
latch = chest.crop((0, 0, 2, 4)).resize((2, 4), Image.NEAREST)
front.alpha_composite(latch, (7, 6))
front.save(f"{DST}/block/chest_front.png")
# side: east lid (28,14) + east body (28,33)
chest_face((28, 14), (28, 33)).save(f"{DST}/block/chest_side.png")
# top: lid top (14,0)
chest.crop((14, 0, 28, 14)).resize((16, 16), Image.NEAREST).save(f"{DST}/block/chest_top.png")
stats["misc"] += 3

# ---------------------------------------------------------------- items -----
MOD_ITEM = f"{MOD}/duneons/textures/item"
MOD_ENT = f"{MOD}/duneons/textures/entities"
CLI_ITEM = f"{CLI}/minecraft/textures/item"

def put(src, dst):
    if not os.path.exists(src):
        print("  !! missing:", src)
        return False
    shutil.copy(src, dst)
    return True

items = [
    # (source path, destination full path)
    (f"{MOD_ITEM}/diamond_sword_dungeon.png", f"{DST}/item/diamond_sword.png"),
    (f"{MOD_ITEM}/iron_sword_variant.png",    f"{DST}/item/iron_sword.png"),
    (f"{MOD_ITEM}/double_axe_new.png",        f"{DST}/item/diamond_axe.png"),
    (f"{MOD_ITEM}/great_hammer.png",          f"{DST}/item/mace.png"),
    (f"{MOD_ITEM}/dagger.png",                f"{DST}/icon_dagger.png"),
    (f"{MOD_ITEM}/gauntlets.png",             f"{DST}/icon_gauntlet.png"),
    (f"{MOD_ITEM}/hunting_bow.png",           f"{DST}/item/bow.png"),
    (f"{CLI_ITEM}/crossbow_standby.png",      f"{DST}/item/crossbow_standby.png"),
    (f"{CLI_ITEM}/arrow.png",                 f"{DST}/item/arrow.png"),
    (f"{MOD_ENT}/emerald.png",                f"{DST}/item/emerald.png"),
    (f"{MOD_ITEM}/burning_potion.png",        f"{DST}/item/potion.png"),
    (f"{MOD_ITEM}/bardsgarb_chestplate.png",  f"{DST}/item/leather_chestplate.png"),
    (f"{MOD_ITEM}/champions_chestplate.png",  f"{DST}/item/iron_chestplate.png"),
    (f"{MOD_ITEM}/full_metal_chestplate.png", f"{DST}/item/netherite_chestplate.png"),
    (f"{MOD_ITEM}/lightning_rod.png",         f"{DST}/art_lightning.png"),
    (f"{MOD_ITEM}/totem_of_regeneration.png", f"{DST}/item/totem_of_undying.png"),
    (f"{MOD_ITEM}/wind_horn.png",             f"{DST}/art_wind.png"),
    (f"{MOD_ITEM}/harvester_icon.png",        f"{DST}/art_harvester.png"),
    (f"{MOD_ITEM}/fireworks.png",             f"{DST}/item/firework_rocket.png"),
    (f"{MOD_ITEM}/t_corruptedseeds_gearicon.png", f"{DST}/art_seeds.png"),
    (f"{MOD_ITEM}/key_icon.png",              f"{DST}/item/trial_key.png"),
    (f"{MOD_ITEM}/ominous_key.png",           f"{DST}/item/golden_key.png"),
]
for src, dst in items:
    if put(src, dst):
        stats["item"] += 1

# MCD heart (animated strip -> first frame)
heart = f"{MOD_ITEM}/heart.png"
if os.path.exists(heart):
    im = Image.open(heart).convert("RGBA")
    if im.height > im.width and im.height % im.width == 0:
        im = im.crop((0, 0, im.width, im.width))
    im.resize((16, 16), Image.NEAREST).save(f"{DST}/icon_heart.png")
    stats["item"] += 1

# ------------------------------------------------------------ player skins --
put(f"{CLI}/minecraft/textures/entity/player/wide/steve.png", f"{SKINS}/steve.png")
put(f"{CLI}/minecraft/textures/entity/player/slim/alex.png", f"{SKINS}/alex.png")
stats["misc"] += 2

# ---------------------------------------------------------------- report ----
n_block = len(glob.glob(f"{DST}/block/*.png"))
n_item = len(glob.glob(f"{DST}/item/*.png"))
n_root = len(glob.glob(f"{DST}/*.png"))
print(f"copied: {stats}")
print(f"total on disk: block={n_block} item={n_item} root={n_root}")

# contact sheet of what the game will now load
preview_keys = ["grass_block_top", "grass_block_side", "dirt", "stone", "cobblestone",
                "mossy_cobblestone", "oak_planks", "oak_log", "oak_log_top", "oak_leaves",
                "sand", "stone_bricks", "mossy_stone_bricks", "cracked_stone_bricks",
                "netherrack", "nether_bricks", "soul_sand", "glowstone", "deepslate",
                "sculk", "chest_front", "chest_side", "chest_top", "gravel", "ice", "snow",
                "diamond_sword", "double_axe_new", "great_hammer", "hunting_bow",
                "emerald", "champions_chestplate", "lightning_rod", "wind_horn",
                "totem_of_regeneration", "fireworks", "key_icon", "burning_potion",
                "dagger", "heart"]
cell = 70
cols = 8
rows = (len(preview_keys) + cols - 1) // cols
sheet = Image.new("RGBA", (cols * cell, rows * (cell + 12)), (28, 28, 38, 255))
from PIL import ImageDraw
d = ImageDraw.Draw(sheet)
for i, k in enumerate(preview_keys):
    for sub in ("block/", "item/", ""):
        p = f"{DST}/{sub}{k}.png"
        if os.path.exists(p):
            break
    else:
        continue
    x, y = (i % cols) * cell, (i // cols) * (cell + 12)
    im = Image.open(p).convert("RGBA")
    # animated strips: first frame
    if im.height > im.width and im.height % im.width == 0:
        im = im.crop((0, 0, im.width, im.width))
    im = im.resize((64, 64), Image.NEAREST)
    sheet.paste(im, (x + 3, y + 2), im)
    d.text((x + 3, y + 66), k[:14], fill=(255, 220, 120, 255))
sheet.save(f"{BASE}/assets/dl/inspect/extracted_preview.png")
print("preview saved")
