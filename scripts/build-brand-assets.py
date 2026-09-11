from pathlib import Path
import sys

from PIL import Image, ImageChops, ImageDraw


def extract_monochrome(source: Image.Image) -> Image.Image:
    """Turn the baked white/checkerboard background into real transparency."""
    rgba = source.convert("RGBA")
    luminance = rgba.convert("L")
    threshold = 220
    ink_alpha = luminance.point(lambda value: 0 if value >= threshold else round((threshold - value) * 255 / threshold))
    ink_alpha = ImageChops.multiply(ink_alpha, rgba.getchannel("A"))
    ink = Image.new("RGBA", rgba.size, (0, 0, 0, 0))
    ink.putalpha(ink_alpha)
    return ink


def fit_square(source: Image.Image, size: int, padding_ratio: float = 0.035) -> Image.Image:
    rgba = extract_monochrome(source)
    alpha_box = rgba.getchannel("A").getbbox()
    if alpha_box:
        rgba = rgba.crop(alpha_box)
    padding = max(1, round(size * padding_ratio))
    available = size - padding * 2
    rgba.thumbnail((available, available), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.alpha_composite(rgba, ((size - rgba.width) // 2, (size - rgba.height) // 2))
    return canvas


def icon_square(source: Image.Image, size: int) -> Image.Image:
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    inset = max(1, round(size * 0.035))
    radius = max(2, round(size * 0.16))
    draw.rounded_rectangle((inset, inset, size - inset, size - inset), radius=radius, fill=(210, 210, 210, 255))
    logo = fit_square(source, size, padding_ratio=0.09)
    canvas.alpha_composite(logo)
    return canvas


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("Usage: build-brand-assets.py SOURCE_PNG PROJECT_ROOT")

    source_path = Path(sys.argv[1]).resolve()
    project_root = Path(sys.argv[2]).resolve()
    public_dir = project_root / "public"
    build_dir = project_root / "build"
    public_dir.mkdir(parents=True, exist_ok=True)
    build_dir.mkdir(parents=True, exist_ok=True)

    source = Image.open(source_path)
    master = fit_square(source, 512)
    master.save(public_dir / "pirate-cinema-logo.png", optimize=True)
    icon = icon_square(source, 512)
    icon_square(source, 64).save(public_dir / "favicon.png", optimize=True)
    icon.save(build_dir / "icon.png", optimize=True)
    icon.save(
        build_dir / "icon.ico",
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )


if __name__ == "__main__":
    main()
