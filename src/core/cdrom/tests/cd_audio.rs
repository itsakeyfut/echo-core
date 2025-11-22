// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! CD-DA (CD Audio) playback tests

use super::super::*;

#[test]
fn test_cd_audio_initialization() {
    let cdrom = CDROM::new();
    assert!(!cdrom.cd_audio.is_playing());
    assert_eq!(cdrom.cd_audio.volume_left, 0x80);
    assert_eq!(cdrom.cd_audio.volume_right, 0x80);
}

#[test]
fn test_cd_audio_playback() {
    let mut cdrom = CDROM::new();

    // Start CD audio playback
    cdrom.cd_audio.play(100, 200, false);
    assert!(cdrom.cd_audio.is_playing());

    // Stop playback
    cdrom.cd_audio.stop();
    assert!(!cdrom.cd_audio.is_playing());
}

#[test]
fn test_cd_audio_volume() {
    let mut cdrom = CDROM::new();

    // Set volume
    cdrom.cd_audio.set_volume(0x40, 0x60);
    assert_eq!(cdrom.cd_audio.volume_left, 0x40);
    assert_eq!(cdrom.cd_audio.volume_right, 0x60);
}

#[test]
fn test_cd_audio_looping() {
    let mut cdrom = CDROM::new();

    // Start looping playback
    cdrom.cd_audio.play(100, 105, true);
    assert!(cdrom.cd_audio.is_playing());
}

#[test]
fn test_cd_audio_samples_when_not_playing() {
    let mut cdrom = CDROM::new();
    let (left, right) = cdrom.cd_audio.get_sample();
    assert_eq!(left, 0);
    assert_eq!(right, 0);
}
