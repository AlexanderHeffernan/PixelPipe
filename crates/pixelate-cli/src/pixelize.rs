use std::path::PathBuf;

use clap::{Args, ValueEnum};
use pixelate_app::{ConvertSelectedReference, convert_selected_reference, pixelization_defaults};
use pixelate_core::{BackdropPolicy, ColorAdjustments, ColorTreatment, ComponentExpectation};
use serde_json::json;

#[derive(Debug, Args)]
pub(crate) struct PixelizeArgs {
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    #[arg(long)]
    pub asset: String,
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(32..=256))]
    pub resolution: u32,
    #[arg(long, default_value_t = 16, value_parser = clap::value_parser!(u8).range(4..=64))]
    pub colors: u8,
    #[arg(long, value_enum, default_value_t = ColorMoodArg::Original)]
    pub mood: ColorMoodArg,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pub brightness: i8,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pub contrast: i8,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pub saturation: i8,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pub warmth: i8,
    #[arg(long, value_enum, default_value_t = BackgroundArg::Auto)]
    pub background: BackgroundArg,
    #[arg(long)]
    pub background_color: Option<String>,
    #[arg(long, default_value_t = 28)]
    pub tolerance: u8,
    #[arg(long, default_value_t = 8)]
    pub alpha_threshold: u8,
    #[arg(long, default_value_t = 1)]
    pub components_min: u16,
    #[arg(long, default_value_t = 32)]
    pub components_max: u16,
    #[arg(long, default_value = "agent")]
    pub actor: String,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ColorMoodArg {
    Original,
    Warm,
    Cool,
    Vivid,
    Muted,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum BackgroundArg {
    Auto,
    None,
    Color,
}

pub(crate) fn pixelize_command(
    options: PixelizeArgs,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut settings = pixelization_defaults().settings;
    settings.width = options.resolution;
    settings.height = options.resolution;
    settings.color_treatment = mood(options.mood);
    settings.color_adjustments = ColorAdjustments {
        brightness: options.brightness,
        contrast: options.contrast,
        saturation: options.saturation,
        warmth: options.warmth,
    };
    settings.components = ComponentExpectation {
        min: options.components_min,
        max: options.components_max,
    };
    let auto_background = matches!(options.background, BackgroundArg::Auto);
    settings.backdrop = match options.background {
        BackgroundArg::Auto => settings.backdrop,
        BackgroundArg::None => BackdropPolicy::Alpha {
            alpha_threshold: options.alpha_threshold,
        },
        BackgroundArg::Color => BackdropPolicy::BorderConnected {
            color: parse_color(
                options
                    .background_color
                    .as_deref()
                    .ok_or("--background color requires --background-color #RRGGBB")?,
            )?,
            tolerance: options.tolerance,
            alpha_threshold: options.alpha_threshold,
        },
    };
    let result = convert_selected_reference(ConvertSelectedReference {
        start: options.root,
        asset: options.asset,
        color_count: Some(options.colors),
        palette_overrides: Vec::new(),
        settings: Some(settings),
        auto_background,
        actor: options.actor,
    })?;
    Ok(json!({ "ok": true, "revision": result }))
}

fn mood(value: ColorMoodArg) -> ColorTreatment {
    match value {
        ColorMoodArg::Original => ColorTreatment::Original,
        ColorMoodArg::Warm => ColorTreatment::Warm,
        ColorMoodArg::Cool => ColorTreatment::Cool,
        ColorMoodArg::Vivid => ColorTreatment::Vivid,
        ColorMoodArg::Muted => ColorTreatment::Muted,
    }
}

fn parse_color(value: &str) -> Result<[u8; 3], String> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid colour '{value}'; expected #RRGGBB"));
    }
    let channel = |start| u8::from_str_radix(&value[start..start + 2], 16).unwrap_or_default();
    Ok([channel(0), channel(2), channel(4)])
}
