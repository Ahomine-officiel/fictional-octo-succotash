#!/usr/bin/env python3
"""Build labeled contact sheets of entity textures from the dungeons mod + client jar."""
from PIL import Image, ImageDraw
import os, math

DL = "/home/z/my-project/mdungeons/assets/dl"
OUT = "/home/z/my-project/mdungeons/assets/dl/inspect"
os.makedirs(OUT, exist_ok=True)

def contact(files, paths, out, cols=6, cell=140):
    rows = math.ceil(len(files) / cols)
    sheet = Image.new("RGBA", (cols * cell, rows * (cell + 18)), (30, 30, 40, 255))
    d = ImageDraw.Draw(sheet)
    for i, f in enumerate(files):
        x, y = (i % cols) * cell, (i // cols) * (cell + 18)
        try:
            im = Image.open(paths(f)).convert("RGBA")
            # checkerboard bg to see alpha
            for cy in range(0, cell, 10):
                for cx in range(0, cell, 10):
                    c = (60, 60, 70, 255) if (cx // 10 + cy // 10) % 2 else (45, 45, 55, 255)
                    d.rectangle([x + cx, y + cy, x + cx + 9, y + cy + 9], fill=c)
            s = min((cell - 8) / im.width, (cell - 8) / im.height)
            im = im.resize((max(1, int(im.width * s)), max(1, int(im.height * s))), Image.NEAREST)
            sheet.paste(im, (x + (cell - im.width) // 2, y + (cell - im.height) // 2), im)
            d.text((x + 4, y + cell), f[:30], fill=(255, 220, 120, 255))
        except Exception as e:
            d.text((x + 4, y + cell), f[:30] + " ERR", fill=(255, 80, 80, 255))
    sheet.save(out)
    print("saved", out, len(files), "tiles")

# MCD mobs sheet 1
mcd1 = ["arch_illager", "ancientguardian", "redstone_monstrosity", "nameless_king_boss",
        "necromancer_boss", "geomancer", "iceologer", "windcaller", "whisperer", "whispererboss",
        "royal_guard", "giant_royal_guard", "skeleton_vangruard", "mossy_skeleton",
        "sunken_skeleton_default", "drowned_necromancer", "ghostly_kindler", "frozen_zombie",
        "endling", "leaper", "poison_anemone", "poison_quill_vine", "blastling", "snareling",
        "cave_crawler", "mini_abomination", "jungle_abomination", "wraith", "wraith_wretched",
        "tempest_golem", "squall_golem", "wildfire", "icy_creeper", "caerbannog", "zombie",
        "jungle_zombie_new", "skeleton_mcd", "enchanted_zombie", "enchanted_creeper",
        "enchanted_vindicator", "mooshroom_enemy", "golem_red", "redstone_golem_noactive",
        "mountaineervariant1", "zombified_pig", "enderman", "endermite", "drowned_variant1",
        "squall_golem_on", "tower_wraith_mde"]
contact(mcd1, lambda f: f"{DL}/x_dungeons/assets/duneons/textures/entities/{f}.png",
        f"{OUT}/mcd_entities.png")

# vanilla mobs from client jar
van = [("zombie/zombie", "zombie"), ("husk", None), ("skeleton/skeleton", "skeleton"),
       ("skeleton/stray", "stray"), ("skeleton/wither_skeleton", "wskel"),
       ("creeper/creeper", "creeper"), ("spider/spider", "spider"),
       ("slime/slime", "slime"), ("slime/magma_cube", "magma"),
       ("pillager", None), ("vindicator", None), ("illager/evoker", "evoker"),
       ("witch", None), ("enderman/enderman", "enderman"), ("endermite", None),
       ("silverfish", None), ("bat", None), ("blaze/blaze", "blaze"), ("ghast/ghast", "ghast"),
       ("vex", None), ("piglin/piglin", "piglin"), ("piglin/piglin_brute", "pbrute"),
       ("hoglin/hoglin", "hoglin"), ("zombified_piglin", None), ("cow/red_mooshroom", "mooshroom"),
       ("spider/cave_spider", "cavespider"), ("zombie/drowned", "drowned")]
def vpath(f):
    name = f[0] if isinstance(f, tuple) else f
    return f"{DL}/x_client/assets/minecraft/textures/entity/{name}.png"
files = [f if isinstance(f, str) else (f[1] or os.path.basename(f[0])) for f in van]
contact(files, vpath, f"{OUT}/van_entities.png", cols=7)
print("done")
