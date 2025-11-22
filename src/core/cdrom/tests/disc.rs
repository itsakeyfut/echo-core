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

//! Disc loading, seeking, and reading tests

use super::super::*;
use tempfile::Builder;

#[test]
fn test_cue_parsing() {
    let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#;

    let tracks = DiscImage::parse_cue(cue_data).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].number, 1);
    assert_eq!(tracks[0].track_type, TrackType::Mode2_2352);
    assert_eq!(tracks[0].start_position.minute, 0);
    assert_eq!(tracks[0].start_position.second, 0);
    assert_eq!(tracks[0].start_position.sector, 0);
}

#[test]
fn test_cue_parsing_multiple_tracks() {
    let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 10:30:15
  TRACK 03 MODE1/2352
    INDEX 01 25:45:20
"#;

    let tracks = DiscImage::parse_cue(cue_data).unwrap();
    assert_eq!(tracks.len(), 3);

    // Track 1
    assert_eq!(tracks[0].number, 1);
    assert_eq!(tracks[0].track_type, TrackType::Mode2_2352);
    assert_eq!(tracks[0].start_position.minute, 0);

    // Track 2
    assert_eq!(tracks[1].number, 2);
    assert_eq!(tracks[1].track_type, TrackType::Audio);
    assert_eq!(tracks[1].start_position.minute, 10);
    assert_eq!(tracks[1].start_position.second, 30);
    assert_eq!(tracks[1].start_position.sector, 15);

    // Track 3
    assert_eq!(tracks[2].number, 3);
    assert_eq!(tracks[2].track_type, TrackType::Mode1_2352);
    assert_eq!(tracks[2].start_position.minute, 25);
    assert_eq!(tracks[2].start_position.second, 45);
    assert_eq!(tracks[2].start_position.sector, 20);
}

#[test]
fn test_msf_to_sector_conversion() {
    let pos = CDPosition {
        minute: 0,
        second: 2,
        sector: 16,
    };
    let sector = DiscImage::msf_to_sector(&pos);
    // 00:02:16 = (2*75 + 16) - 150 pregap = 16
    assert_eq!(sector, 16);

    let pos = CDPosition {
        minute: 1,
        second: 0,
        sector: 0,
    };
    let sector = DiscImage::msf_to_sector(&pos);
    // 01:00:00 = 60*75 - 150 pregap = 4350
    assert_eq!(sector, 4350);
}

#[test]
fn test_parse_msf() {
    let pos = DiscImage::parse_msf("10:30:15").unwrap();
    assert_eq!(pos.minute, 10);
    assert_eq!(pos.second, 30);
    assert_eq!(pos.sector, 15);

    let pos = DiscImage::parse_msf("00:00:00").unwrap();
    assert_eq!(pos.minute, 0);
    assert_eq!(pos.second, 0);
    assert_eq!(pos.sector, 0);
}

#[test]
fn test_parse_msf_invalid() {
    // Invalid format - only 2 components
    assert!(DiscImage::parse_msf("10:30").is_err());

    // Invalid format - 4 components
    assert!(DiscImage::parse_msf("10:30:15:00").is_err());

    // Invalid numbers
    assert!(DiscImage::parse_msf("abc:def:ghi").is_err());
}

#[test]
fn test_parse_track_type() {
    assert_eq!(
        DiscImage::parse_track_type("MODE1/2352"),
        TrackType::Mode1_2352
    );
    assert_eq!(
        DiscImage::parse_track_type("MODE2/2352"),
        TrackType::Mode2_2352
    );
    assert_eq!(DiscImage::parse_track_type("AUDIO"), TrackType::Audio);

    // Unknown type defaults to Mode2
    assert_eq!(
        DiscImage::parse_track_type("UNKNOWN"),
        TrackType::Mode2_2352
    );
}

#[test]
fn test_disc_image_with_mock_data() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_disc_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_disc_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    // Create a mock .cue file with proper pregap (starts at 00:02:00)
    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    // Create a mock .bin file (10 sectors = 23520 bytes)
    let sector_data = vec![0xAB; 2352];
    let mut bin_data = Vec::new();
    for _ in 0..10 {
        bin_data.extend_from_slice(&sector_data);
    }
    std::fs::write(bin_path, &bin_data).unwrap();

    // Load the disc image
    let disc = DiscImage::load(cue_path.to_str().unwrap()).unwrap();

    // Verify track count
    assert_eq!(disc.track_count(), 1);

    // Verify track info
    let track = disc.get_track(1).unwrap();
    assert_eq!(track.number, 1);
    assert_eq!(track.track_type, TrackType::Mode2_2352);
    assert_eq!(track.length_sectors, 10);

    // Read first sector at LBA 0 (MSF 00:02:00)
    let pos = CDPosition::new(0, 2, 0);
    let sector = disc.read_sector(&pos).unwrap();
    assert_eq!(sector.len(), 2352);
    assert_eq!(sector[0], 0xAB);

    // Read last sector at LBA 9 (MSF 00:02:09)
    let pos = CDPosition::new(0, 2, 9);
    let sector = disc.read_sector(&pos).unwrap();
    assert_eq!(sector.len(), 2352);

    // Read out of bounds (LBA 10, MSF 00:02:10)
    let pos = CDPosition::new(0, 2, 10);
    assert!(disc.read_sector(&pos).is_none());

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_cdrom_load_disc() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_load_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_load_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    // Create mock files with proper pregap
    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let bin_data = vec![0x00; 2352 * 5]; // 5 sectors
    std::fs::write(bin_path, &bin_data).unwrap();

    // Load disc into CDROM
    let mut cdrom = CDROM::new();
    assert!(!cdrom.has_disc());

    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    assert!(cdrom.has_disc());
    assert!(!cdrom.status.shell_open);

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_cdrom_read_current_sector() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_read_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_read_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    // Create mock files with recognizable pattern and proper pregap
    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let mut bin_data = Vec::new();
    for i in 0..5 {
        let mut sector = vec![i as u8; 2352];
        bin_data.append(&mut sector);
    }
    std::fs::write(bin_path, &bin_data).unwrap();

    // Load and read
    let mut cdrom = CDROM::new();
    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    // Read sector at position 00:02:00 (LBA 0)
    cdrom.set_position(CDPosition::new(0, 2, 0));
    let sector = cdrom.read_current_sector().unwrap();
    assert_eq!(sector.len(), 2352);
    assert_eq!(sector[0], 0); // First sector filled with 0

    // Read sector at position 00:02:03 (LBA 3)
    cdrom.set_position(CDPosition::new(0, 2, 3));
    let sector = cdrom.read_current_sector().unwrap();
    assert_eq!(sector[0], 3); // Fourth sector filled with 3

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_cdrom_read_without_disc() {
    let mut cdrom = CDROM::new();
    assert!(cdrom.read_current_sector().is_none());
}

#[test]
fn test_sector_reading() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_sector_reading_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_sector_reading_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    // Create mock .cue file with proper pregap
    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    // Create mock .bin file with recognizable pattern (10 sectors)
    let mut bin_data = Vec::new();
    for i in 0..10 {
        let mut sector = vec![i as u8; 2352];
        bin_data.append(&mut sector);
    }
    std::fs::write(bin_path, &bin_data).unwrap();

    // Load disc into CDROM
    let mut cdrom = CDROM::new();
    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    // Set position to start at LBA 0 (MSF 00:02:00)
    cdrom.set_position(CDPosition::new(0, 2, 0));

    // Start reading
    cdrom.execute_command(0x06); // ReadN

    // Verify reading state
    assert_eq!(cdrom.state, CDState::Reading);
    assert!(cdrom.status.reading);

    // Initially, no data buffer should be present
    assert!(cdrom.data_buffer.is_empty());

    // Tick until first sector ready (need exactly 13,300 cycles)
    cdrom.tick(13_300);

    // Should have data now
    assert!(!cdrom.data_buffer.is_empty());
    assert_eq!(cdrom.data_buffer.len(), 2352);
    assert_eq!(cdrom.data_buffer[0], 0); // First sector filled with 0

    // Check that interrupt was triggered (INT1 - data ready)
    assert_ne!(cdrom.interrupt_flag & 0x01, 0);

    // Position should have advanced to next sector (00:02:01)
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 1);

    // Clear interrupt and continue reading
    cdrom.acknowledge_interrupt(0x01);

    // Tick until next sector (exactly 13,300 more cycles)
    cdrom.tick(13_300);

    // Should have second sector data
    assert_eq!(cdrom.data_buffer[0], 1); // Second sector filled with 1
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 2);

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_seek_timing() {
    let mut cdrom = CDROM::new();

    // Set seek target
    cdrom.seek_target = Some(CDPosition::new(0, 10, 30));

    // Start seek
    cdrom.execute_command(0x15); // SeekL

    // Should be in seeking state
    assert_eq!(cdrom.state, CDState::Seeking);
    assert!(cdrom.status.seeking);

    // Should have INT3 (acknowledge)
    assert_ne!(cdrom.interrupt_flag & 0x04, 0);
    cdrom.acknowledge_interrupt(0x04);

    // Position should not have changed yet
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 0);

    // Tick until seek completes (need ~100,000 cycles)
    for _ in 0..120_000 {
        cdrom.tick(1);
    }

    // Seek should be complete
    assert_eq!(cdrom.state, CDState::Idle);
    assert!(!cdrom.status.seeking);

    // Position should have changed to target
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 10);
    assert_eq!(cdrom.position.sector, 30);

    // Should have INT2 (complete)
    assert_ne!(cdrom.interrupt_flag & 0x02, 0);
}

#[test]
fn test_track_length_calculation_realistic() {
    let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
  TRACK 02 AUDIO
    INDEX 01 00:03:00
"#;

    let mut tracks = DiscImage::parse_cue(cue_data).unwrap();

    // Track 1 starts at 00:02:00 (LBA 0), track 2 starts at 00:03:00 (LBA 75)
    // Total file size: 150 sectors
    DiscImage::calculate_track_lengths(&mut tracks, 2352 * 150);

    // Track 1: 75 sectors (LBA 0-74, file offset 0 to 75*2352)
    assert_eq!(tracks[0].length_sectors, 75);

    // Track 2: 75 sectors (LBA 75-149, file offset 75*2352 to 150*2352)
    assert_eq!(tracks[1].length_sectors, 75);
}

#[test]
fn test_get_track() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_get_track_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_get_track_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
  TRACK 02 AUDIO
    INDEX 01 00:03:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let bin_data = vec![0x00; 2352 * 150];
    std::fs::write(bin_path, &bin_data).unwrap();

    let disc = DiscImage::load(cue_path.to_str().unwrap()).unwrap();

    // Get track 1
    let track1 = disc.get_track(1);
    assert!(track1.is_some());
    assert_eq!(track1.unwrap().number, 1);
    assert_eq!(track1.unwrap().track_type, TrackType::Mode2_2352);

    // Get track 2
    let track2 = disc.get_track(2);
    assert!(track2.is_some());
    assert_eq!(track2.unwrap().number, 2);
    assert_eq!(track2.unwrap().track_type, TrackType::Audio);

    // Get non-existent track
    let track99 = disc.get_track(99);
    assert!(track99.is_none());

    // Files automatically cleaned up when tempfile goes out of scope
}
