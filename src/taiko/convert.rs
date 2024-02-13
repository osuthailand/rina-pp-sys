use std::borrow::Cow;

use rosu_map::{section::general::GameMode, util::Pos};

use crate::{
    model::{
        beatmap::{get_precision_adjusted_beat_len_taiko_mania, Beatmap, Converted},
        control_point::TimingPoint,
        hit_object::{HitObject, HitObjectKind, HoldNote, Slider, Spinner},
        mode::ConvertStatus,
    },
    util::{float_ext::FloatExt, tandem_sort::TandemSorter},
};

use super::Taiko;

/// A [`Beatmap`] for [`Taiko`] calculations.
pub type TaikoBeatmap<'a> = Converted<'a, Taiko>;

const LEGACY_TAIKO_VELOCITY_MULTIPLIER: f32 = 1.4;
const OSU_BASE_SCORING_DIST: f32 = 100.0;

pub fn try_convert(map: &mut Cow<'_, Beatmap>) -> ConvertStatus {
    match map.mode {
        GameMode::Osu => {
            convert(map.to_mut());

            ConvertStatus::Done
        }
        GameMode::Taiko => ConvertStatus::Noop,
        GameMode::Catch | GameMode::Mania => ConvertStatus::Incompatible,
    }
}

fn convert(map: &mut Beatmap) {
    let mut new_objects = Vec::new();
    let mut new_sounds = Vec::new();

    let mut idx = 0;

    while idx < map.hit_objects.len() {
        match map.hit_objects[idx].kind {
            HitObjectKind::Circle | HitObjectKind::Spinner(_) => {}
            HitObjectKind::Slider(ref slider) => {
                let obj = &map.hit_objects[idx];
                let mut params = SliderParams::new(obj.start_time, slider);

                if should_convert_slider_to_taiko_hits(map, &mut params) {
                    let mut i = 0;
                    let mut j = obj.start_time;

                    let edge_sound_count = slider.node_sounds.len().max(1);

                    while j
                        <= obj.start_time + f64::from(params.duration) + params.tick_spacing / 8.0
                    {
                        let h = HitObject {
                            pos: Pos::default(),
                            start_time: j,
                            kind: HitObjectKind::Circle,
                        };

                        new_objects.push(h);
                        new_sounds.push(
                            slider
                                .node_sounds
                                .get(i)
                                .copied()
                                .unwrap_or(map.hit_sounds[idx]),
                        );

                        if params.tick_spacing.eq(0.0) {
                            break;
                        }

                        j += params.tick_spacing;
                        i = (i + 1) % edge_sound_count;
                    }

                    if let Some(len) = new_objects.len().checked_sub(1) {
                        map.hit_objects.splice(idx..=idx, new_objects.drain(..));
                        map.hit_sounds.splice(idx..=idx, new_sounds.drain(..));
                        idx += len;
                    } else {
                        map.hit_objects.remove(idx);
                        map.hit_sounds.remove(idx);
                        idx -= 1;
                    }
                }
            }
            // Pathological case; shouldn't realistically happen
            HitObjectKind::Hold(HoldNote { end_time }) => {
                map.hit_objects[idx].kind = HitObjectKind::Spinner(Spinner { end_time });
            }
        }

        idx += 1;
    }

    // We only convert osu! to taiko so we don't need to remove objects
    // with the same timestamp that would appear only in mania

    let mut sorter = TandemSorter::new(
        &map.hit_objects,
        |a, b| a.start_time.total_cmp(&b.start_time),
        true,
    );
    sorter.sort(&mut map.hit_objects);
    sorter.sort(&mut map.hit_sounds);

    map.mode = GameMode::Taiko;
}

fn should_convert_slider_to_taiko_hits(map: &Beatmap, params: &mut SliderParams<'_>) -> bool {
    let SliderParams {
        slider,
        duration,
        start_time,
        tick_spacing,
    } = params;

    // * The true distance, accounting for any repeats. This ends up being the drum roll distance later
    let spans = slider.span_count() as f64;
    let mut dist = slider.expected_dist.unwrap_or(0.0);

    dist *= f64::from(LEGACY_TAIKO_VELOCITY_MULTIPLIER);
    dist *= spans;

    let timing_beat_len = map
        .timing_point_at(*start_time)
        .map_or(TimingPoint::DEFAULT_BEAT_LEN, |point| point.beat_len);

    let mut beat_len =
        get_precision_adjusted_beat_len_taiko_mania(map, timing_beat_len, *start_time);

    let slider_scoring_point_dist = f64::from(OSU_BASE_SCORING_DIST)
        * (map.slider_multiplier * f64::from(LEGACY_TAIKO_VELOCITY_MULTIPLIER))
        / map.slider_tick_rate;

    // * The velocity and duration of the taiko hit object - calculated as the velocity of a drum roll.
    let taiko_vel = slider_scoring_point_dist * map.slider_tick_rate;
    *duration = (dist / taiko_vel * beat_len) as u32;

    let osu_vel = taiko_vel * (f64::from(1000.0_f32) / beat_len);

    // * osu-stable always uses the speed-adjusted beatlength to determine the osu! velocity, but only uses it for conversion if beatmap version < 8
    if map.version >= 8 {
        beat_len = timing_beat_len;
    }

    // * If the drum roll is to be split into hit circles, assume the ticks are 1/8 spaced within the duration of one beat
    *tick_spacing = (beat_len / map.slider_tick_rate).min(f64::from(*duration) / spans);

    *tick_spacing > 0.0 && dist / osu_vel * 1000.0 < 2.0 * beat_len
}

struct SliderParams<'c> {
    slider: &'c Slider,
    duration: u32,
    start_time: f64,
    tick_spacing: f64,
}

impl<'c> SliderParams<'c> {
    const fn new(start_time: f64, slider: &'c Slider) -> Self {
        Self {
            slider,
            start_time,
            duration: 0,
            tick_spacing: 0.0,
        }
    }
}