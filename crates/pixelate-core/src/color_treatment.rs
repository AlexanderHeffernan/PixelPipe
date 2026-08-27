use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTreatment {
    #[default]
    Original,
    Warm,
    Cool,
    Vivid,
    Muted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorAdjustments {
    #[serde(default)]
    pub brightness: i8,
    #[serde(default)]
    pub contrast: i8,
    #[serde(default)]
    pub saturation: i8,
    #[serde(default)]
    pub warmth: i8,
}

impl ColorAdjustments {
    #[must_use]
    pub const fn is_neutral(&self) -> bool {
        self.brightness == 0 && self.contrast == 0 && self.saturation == 0 && self.warmth == 0
    }

    pub(crate) fn apply(self, pixel: [u8; 4]) -> [u8; 4] {
        let [red, green, blue, alpha] = pixel;
        let brightness = i32::from(self.brightness) * 255 / 200;
        let contrast = 100 + i32::from(self.contrast);
        let warmth = i32::from(self.warmth) * 48 / 100;
        let adjusted = [
            channel(red, brightness + warmth, contrast),
            channel(green, brightness, contrast),
            channel(blue, brightness - warmth, contrast),
        ];
        let saturated = saturate(adjusted, 100 + i32::from(self.saturation));
        [saturated[0], saturated[1], saturated[2], alpha]
    }
}

impl ColorTreatment {
    pub(crate) fn apply(self, pixel: [u8; 4]) -> [u8; 4] {
        let [red, green, blue, alpha] = pixel;
        let rgb = match self {
            Self::Original => [red, green, blue],
            Self::Warm => [
                toward_white(red, 12),
                toward_white(green, 3),
                scale(blue, 88),
            ],
            Self::Cool => [
                scale(red, 90),
                toward_white(green, 3),
                toward_white(blue, 12),
            ],
            Self::Vivid => saturate([red, green, blue], 135),
            Self::Muted => saturate([red, green, blue], 65),
        };
        [rgb[0], rgb[1], rgb[2], alpha]
    }

    #[must_use]
    pub const fn is_original(&self) -> bool {
        matches!(self, Self::Original)
    }
}

fn channel(value: u8, brightness: i32, contrast: i32) -> u8 {
    let value = ((i32::from(value) - 128) * contrast / 100) + 128 + brightness;
    u8::try_from(value.clamp(0, 255)).unwrap_or_default()
}

fn toward_white(value: u8, percent: u16) -> u8 {
    let value = u16::from(value);
    u8::try_from(value + (255 - value) * percent / 100).unwrap_or(u8::MAX)
}

fn scale(value: u8, percent: u16) -> u8 {
    u8::try_from((u16::from(value) * percent / 100).min(255)).unwrap_or(u8::MAX)
}

fn saturate(rgb: [u8; 3], percent: i32) -> [u8; 3] {
    let luma = (77 * i32::from(rgb[0]) + 150 * i32::from(rgb[1]) + 29 * i32::from(rgb[2])) / 256;
    rgb.map(|value| {
        let adjusted = luma + (i32::from(value) - luma) * percent / 100;
        u8::try_from(adjusted.clamp(0, 255)).unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treatments_are_deterministic_and_preserve_alpha() {
        let source = [80, 140, 210, 117];
        assert_eq!(ColorTreatment::Original.apply(source), source);
        assert_eq!(ColorTreatment::Warm.apply(source)[3], 117);
        assert!(ColorTreatment::Vivid.apply(source)[2] > source[2]);
        assert!(ColorTreatment::Muted.apply(source)[2] < source[2]);
    }

    #[test]
    fn fine_adjustments_are_deterministic_and_neutral_by_default() {
        let source = [80, 140, 210, 117];
        assert_eq!(ColorAdjustments::default().apply(source), source);
        let adjusted = ColorAdjustments {
            brightness: 20,
            contrast: 10,
            saturation: 25,
            warmth: 30,
        }
        .apply(source);
        assert_eq!(adjusted[3], source[3]);
        assert!(adjusted[0] > source[0]);
    }
}
