//! Which modem presets each region allows.
//!
//! Firmware 2.8 groups regions into profiles, and each profile allows only its
//! own presets: `NARROW_FAST` belongs to EU_N_868, `LITE_FAST` to EU_866, and
//! the tiny presets to the amateur bands. Picking a preset that the region does
//! not allow produces a configuration the radio refuses.
//!
//! This mirrors the region table and `regionSwapForPreset()` in the firmware's
//! `RadioInterface.cpp`.

use crate::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};

/// Presets available in most regions.
#[allow(deprecated)]
const PRESETS_STD: &[ModemPreset] = &[
    ModemPreset::LongFast,
    ModemPreset::LongSlow,
    ModemPreset::MediumSlow,
    ModemPreset::MediumFast,
    ModemPreset::ShortSlow,
    ModemPreset::ShortFast,
    ModemPreset::LongModerate,
    ModemPreset::ShortTurbo,
    ModemPreset::LongTurbo,
    ModemPreset::MediumTurbo,
];

/// EU 868 has the standard presets minus the turbo ones, which are too wide
/// for its narrow allocation.
#[allow(deprecated)]
const PRESETS_EU_868: &[ModemPreset] = &[
    ModemPreset::LongFast,
    ModemPreset::LongSlow,
    ModemPreset::MediumSlow,
    ModemPreset::MediumFast,
    ModemPreset::ShortSlow,
    ModemPreset::ShortFast,
    ModemPreset::LongModerate,
];

/// EU 866, the non-specific short range band.
const PRESETS_LITE: &[ModemPreset] = &[ModemPreset::LiteFast, ModemPreset::LiteSlow];

/// EU narrow 868, and the amateur 70cm and 1.25m bands.
const PRESETS_NARROW: &[ModemPreset] = &[ModemPreset::NarrowFast, ModemPreset::NarrowSlow];

/// The amateur 2m bands, at 15.6 kHz.
const PRESETS_TINY: &[ModemPreset] = &[ModemPreset::TinyFast, ModemPreset::TinySlow];

/// An unconfigured region offers nothing but the default.
const PRESETS_UNSET: &[ModemPreset] = &[ModemPreset::LongFast];

/// The presets `region` allows.
pub fn presets_for_region(region: RegionCode) -> &'static [ModemPreset] {
    match region {
        RegionCode::Eu868 => PRESETS_EU_868,
        RegionCode::Eu866 => PRESETS_LITE,
        RegionCode::EuN868 | RegionCode::Itu170cm | RegionCode::Itu270cm | RegionCode::Itu370cm => {
            PRESETS_NARROW
        }
        RegionCode::Itu2125cm => PRESETS_NARROW,
        RegionCode::Itu12m | RegionCode::Itu22m | RegionCode::Itu32m => PRESETS_TINY,
        RegionCode::Unset => PRESETS_UNSET,
        _ => PRESETS_STD,
    }
}

/// Whether `region` allows `preset`.
pub fn region_supports_preset(region: RegionCode, preset: ModemPreset) -> bool {
    presets_for_region(region).contains(&preset)
}

/// The EU regions that swap between each other when a preset asks for it.
///
/// Their preset lists do not overlap, so choosing a preset that belongs to a
/// sibling means the user wants that sibling region.
const SWAPPABLE_EU_REGIONS: &[RegionCode] =
    &[RegionCode::Eu868, RegionCode::Eu866, RegionCode::EuN868];

/// The region to switch to so that `preset` becomes legal.
///
/// Returns `None` when the current region already allows the preset, or when
/// no sibling region would help. Mirrors `regionSwapForPreset()` in the
/// firmware: selecting NARROW_FAST while on EU_868 means EU_N_868.
pub fn region_swap_for_preset(region: RegionCode, preset: ModemPreset) -> Option<RegionCode> {
    if region_supports_preset(region, preset) {
        return None;
    }

    if !SWAPPABLE_EU_REGIONS.contains(&region) {
        return None;
    }

    SWAPPABLE_EU_REGIONS
        .iter()
        .find(|sibling| **sibling != region && region_supports_preset(**sibling, preset))
        .copied()
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn narrow_presets_belong_to_the_narrow_eu_region() {
        assert!(region_supports_preset(
            RegionCode::EuN868,
            ModemPreset::NarrowFast
        ));
        assert!(!region_supports_preset(
            RegionCode::Eu868,
            ModemPreset::NarrowFast
        ));
    }

    #[test]
    fn choosing_a_narrow_preset_on_eu_868_swaps_to_eu_n_868() {
        // The case that prompted this: the preset requires the sibling region.
        assert_eq!(
            region_swap_for_preset(RegionCode::Eu868, ModemPreset::NarrowFast),
            Some(RegionCode::EuN868)
        );
        assert_eq!(
            region_swap_for_preset(RegionCode::Eu868, ModemPreset::NarrowSlow),
            Some(RegionCode::EuN868)
        );
    }

    #[test]
    fn choosing_a_lite_preset_swaps_to_eu_866() {
        assert_eq!(
            region_swap_for_preset(RegionCode::Eu868, ModemPreset::LiteFast),
            Some(RegionCode::Eu866)
        );
        assert_eq!(
            region_swap_for_preset(RegionCode::EuN868, ModemPreset::LiteSlow),
            Some(RegionCode::Eu866)
        );
    }

    #[test]
    fn a_standard_preset_swaps_back_to_eu_868() {
        assert_eq!(
            region_swap_for_preset(RegionCode::EuN868, ModemPreset::LongFast),
            Some(RegionCode::Eu868)
        );
    }

    #[test]
    fn no_swap_when_the_region_already_allows_the_preset() {
        assert_eq!(
            region_swap_for_preset(RegionCode::Eu868, ModemPreset::LongFast),
            None
        );
        assert_eq!(
            region_swap_for_preset(RegionCode::EuN868, ModemPreset::NarrowFast),
            None
        );
    }

    #[test]
    fn no_swap_outside_the_eu_trio() {
        // The firmware only swaps between the three EU regions; anywhere else
        // an illegal pairing stays illegal.
        assert_eq!(
            region_swap_for_preset(RegionCode::Us, ModemPreset::NarrowFast),
            None
        );
        assert_eq!(
            region_swap_for_preset(RegionCode::Itu170cm, ModemPreset::LongFast),
            None
        );
    }

    #[test]
    fn the_turbo_presets_are_too_wide_for_eu_868() {
        assert!(region_supports_preset(
            RegionCode::Us,
            ModemPreset::ShortTurbo
        ));
        assert!(!region_supports_preset(
            RegionCode::Eu868,
            ModemPreset::ShortTurbo
        ));
    }

    #[test]
    fn the_amateur_bands_carry_their_own_presets() {
        assert!(region_supports_preset(
            RegionCode::Itu12m,
            ModemPreset::TinyFast
        ));
        assert!(region_supports_preset(
            RegionCode::Itu170cm,
            ModemPreset::NarrowFast
        ));
        assert!(!region_supports_preset(
            RegionCode::Itu12m,
            ModemPreset::LongFast
        ));
    }

    #[test]
    fn every_region_allows_at_least_one_preset() {
        for region in crate::models::REGION_CODES {
            assert!(
                !presets_for_region(*region).is_empty(),
                "{:?} allows nothing",
                region
            );
        }
    }
}
