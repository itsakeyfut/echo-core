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

//! Tests for CD-ROM emulation

use super::*;
use tempfile::Builder;

#[test]
fn test_cdrom_initialization() {
    let cdrom = CDROM::new();
    assert_eq!(cdrom.state, CDState::Idle);
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 0);
}

#[test]
fn test_getstat() {
    let mut cdrom = CDROM::new();
    cdrom.execute_command(0x01);

    assert!(!cdrom.response_fifo.is_empty());
    assert_ne!(cdrom.interrupt_flag, 0);

    // Check INT3 was triggered
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
}

#[test]
fn test_setloc() {
    let mut cdrom = CDROM::new();

    // Set parameters: 00:02:00 in BCD
    cdrom.param_fifo.push_back(0x00); // MM
    cdrom.param_fifo.push_back(0x02); // SS
    cdrom.param_fifo.push_back(0x00); // FF

    cdrom.execute_command(0x02);

    assert!(cdrom.seek_target.is_some());
    let target = cdrom.seek_target.unwrap();
    assert_eq!(target.minute, 0);
    assert_eq!(target.second, 2);
    assert_eq!(target.sector, 0);
}

#[test]
fn test_setloc_insufficient_params() {
    let mut cdrom = CDROM::new();

    // Only push 2 parameters (need 3)
    cdrom.param_fifo.push_back(0x00);
    cdrom.param_fifo.push_back(0x02);

    cdrom.execute_command(0x02);

    // Should get error response
    assert_eq!(cdrom.interrupt_flag & 0x10, 0x10); // INT5
}

#[test]
fn test_seekl() {
    let mut cdrom = CDROM::new();

    // Set target first
    cdrom.seek_target = Some(CDPosition::new(0, 10, 30));

    cdrom.execute_command(0x15); // SeekL

    // Position should NOT be updated yet (seek takes time)
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 0);

    // Should be in seeking state
    assert_eq!(cdrom.state, CDState::Seeking);

    // Should have responses and interrupts (INT3 acknowledge)
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_init() {
    let mut cdrom = CDROM::new();
    cdrom.execute_command(0x0A); // Init

    assert!(cdrom.status.motor_on);
    assert_eq!(cdrom.state, CDState::Idle);
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_readn() {
    let mut cdrom = CDROM::new();
    cdrom.execute_command(0x06); // ReadN

    assert_eq!(cdrom.state, CDState::Reading);
    assert!(cdrom.status.reading);
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_pause() {
    let mut cdrom = CDROM::new();

    // Start reading first
    cdrom.state = CDState::Reading;
    cdrom.status.reading = true;

    cdrom.execute_command(0x09); // Pause

    assert_eq!(cdrom.state, CDState::Idle);
    assert!(!cdrom.status.reading);
}

#[test]
fn test_unknown_command() {
    let mut cdrom = CDROM::new();
    cdrom.execute_command(0xFF); // Invalid command

    // Should trigger error interrupt (INT5)
    assert_eq!(cdrom.interrupt_flag & 0x10, 0x10);
}

#[test]
fn test_bcd_conversion() {
    assert_eq!(bcd_to_dec(0x23), 23);
    assert_eq!(bcd_to_dec(0x00), 0);
    assert_eq!(bcd_to_dec(0x99), 99);

    assert_eq!(dec_to_bcd(23), 0x23);
    assert_eq!(dec_to_bcd(0), 0x00);
    assert_eq!(dec_to_bcd(99), 0x99);
}

#[test]
fn test_msf_to_lba() {
    let pos = CDPosition::new(0, 2, 0);
    assert_eq!(pos.to_lba(), 0); // Start of data (after 2-second pregap)

    let pos = CDPosition::new(0, 3, 0);
    assert_eq!(pos.to_lba(), 75); // 1 second after start
}

#[test]
fn test_lba_to_msf() {
    let pos = CDPosition::from_lba(0);
    assert_eq!(pos.minute, 0);
    assert_eq!(pos.second, 2);
    assert_eq!(pos.sector, 0);

    let pos = CDPosition::from_lba(75);
    assert_eq!(pos.minute, 0);
    assert_eq!(pos.second, 3);
    assert_eq!(pos.sector, 0);
}

#[test]
fn test_status_byte() {
    let mut cdrom = CDROM::new();

    // Initial status
    let status = cdrom.get_status_byte();
    assert_eq!(status, 0);

    // Set motor on
    cdrom.status.motor_on = true;
    let status = cdrom.get_status_byte();
    assert_eq!(status & 0x02, 0x02);

    // Set reading
    cdrom.status.reading = true;
    let status = cdrom.get_status_byte();
    assert_eq!(status & 0x20, 0x20);
}

#[test]
fn test_interrupt_acknowledge() {
    let mut cdrom = CDROM::new();

    cdrom.trigger_interrupt(3);
    assert_eq!(cdrom.interrupt_flag, 0x04);

    cdrom.acknowledge_interrupt(0x04);
    assert_eq!(cdrom.interrupt_flag, 0x00);
}

#[test]
fn test_param_response_fifos() {
    let mut cdrom = CDROM::new();

    // Test parameter FIFO
    cdrom.push_param(0x12);
    cdrom.push_param(0x34);
    assert_eq!(cdrom.param_fifo.len(), 2);

    // Test response FIFO
    cdrom.execute_command(0x01); // GetStat
    assert!(!cdrom.response_empty());

    let response = cdrom.pop_response();
    assert!(response.is_some());
}

#[test]
fn test_status_register() {
    let mut cdrom = CDROM::new();

    // Initial status: parameter FIFO empty and not full
    let status = cdrom.read_status();
    assert_eq!(status & 0x08, 0x08); // Bit 3: Parameter FIFO empty
    assert_eq!(status & 0x10, 0x10); // Bit 4: Parameter FIFO not full
    assert_eq!(status & 0x20, 0x00); // Bit 5: Response FIFO empty
    assert_eq!(status & 0x80, 0x00); // Bit 7: Not busy

    // Push parameter - FIFO should no longer be empty
    cdrom.push_param(0x12);
    let status = cdrom.read_status();
    assert_eq!(status & 0x08, 0x00); // Bit 3: Parameter FIFO not empty
    assert_eq!(status & 0x10, 0x10); // Bit 4: Parameter FIFO still not full

    // Execute command - response FIFO should have data
    cdrom.execute_command(0x01); // GetStat
    let status = cdrom.read_status();
    assert_eq!(status & 0x20, 0x20); // Bit 5: Response FIFO not empty

    // Set seeking state - should show busy
    cdrom.state = CDState::Seeking;
    let status = cdrom.read_status();
    assert_eq!(status & 0x80, 0x80); // Bit 7: Busy
}

#[test]
fn test_status_register_ready_state() {
    let cdrom = CDROM::new();

    // On initialization, CDROM should report ready state (0x18)
    // Bit 3 (Parameter FIFO empty) = 1
    // Bit 4 (Parameter FIFO not full) = 1
    let status = cdrom.read_status();
    assert_eq!(status & 0x18, 0x18); // Ready state bits
}

// Disc Image Loading Tests

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
fn test_cdrom_position_accessors() {
    let mut cdrom = CDROM::new();

    // Check initial position
    let pos = cdrom.position();
    assert_eq!(pos.minute, 0);
    assert_eq!(pos.second, 2);
    assert_eq!(pos.sector, 0);

    // Set new position
    cdrom.set_position(CDPosition::new(10, 30, 15));
    let pos = cdrom.position();
    assert_eq!(pos.minute, 10);
    assert_eq!(pos.second, 30);
    assert_eq!(pos.sector, 15);
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
fn test_advance_position() {
    let mut cdrom = CDROM::new();

    // Test normal sector advancement
    cdrom.set_position(CDPosition::new(0, 2, 0));
    cdrom.advance_position();
    assert_eq!(cdrom.position.sector, 1);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.minute, 0);

    // Test sector wraparound (74 -> 0, second++)
    cdrom.set_position(CDPosition::new(0, 2, 74));
    cdrom.advance_position();
    assert_eq!(cdrom.position.sector, 0);
    assert_eq!(cdrom.position.second, 3);
    assert_eq!(cdrom.position.minute, 0);

    // Test second wraparound (59 -> 0, minute++)
    cdrom.set_position(CDPosition::new(0, 59, 74));
    cdrom.advance_position();
    assert_eq!(cdrom.position.sector, 0);
    assert_eq!(cdrom.position.second, 0);
    assert_eq!(cdrom.position.minute, 1);
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
fn test_get_data_byte() {
    let mut cdrom = CDROM::new();

    // Set up data buffer with test pattern
    cdrom.data_buffer = vec![0x11, 0x22, 0x33, 0x44, 0x55];
    cdrom.data_index = 0;

    // Read bytes sequentially
    assert_eq!(cdrom.get_data_byte(), 0x11);
    assert_eq!(cdrom.get_data_byte(), 0x22);
    assert_eq!(cdrom.get_data_byte(), 0x33);
    assert_eq!(cdrom.get_data_byte(), 0x44);
    assert_eq!(cdrom.get_data_byte(), 0x55);

    // Reading beyond buffer should return 0
    assert_eq!(cdrom.get_data_byte(), 0);
    assert_eq!(cdrom.get_data_byte(), 0);
}

#[test]
fn test_register_read_write() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    // Register timing events
    cdrom.register_events(&mut timing);

    // Test status register read
    let status = cdrom.read_register(CDROM::REG_INDEX);
    assert_eq!(status & 0x18, 0x18); // FIFO empty and not full

    // Test index selection
    cdrom.write_register(CDROM::REG_INDEX, 2);
    assert_eq!(cdrom.index, 2);

    // Test parameter write (index 0)
    cdrom.write_register(CDROM::REG_INDEX, 0);
    cdrom.write_register(CDROM::REG_INT_FLAG, 0x42);
    assert_eq!(cdrom.param_fifo.len(), 1);
    assert_eq!(cdrom.param_fifo[0], 0x42);

    // Set motor on so status byte is non-zero
    cdrom.status.motor_on = true;

    // Test command write (index 0) - now uses timing system
    cdrom.write_register(CDROM::REG_DATA, 0x01); // GetStat command is queued

    // Command should be queued but not executed yet
    assert!(cdrom.response_fifo.is_empty());
    assert!(cdrom.command_to_schedule.is_some());

    // Process events to schedule the command
    cdrom.process_events(&mut timing, &[]);

    // Command is now scheduled, advance timing to execute it
    timing.pending_ticks = 10_000; // More than ACK delay (5000 cycles)
    let triggered = timing.run_events();

    // Process the triggered event
    cdrom.process_events(&mut timing, &triggered);

    // Now response should be available
    assert!(!cdrom.response_fifo.is_empty());

    // Test response read (index 0)
    let response = cdrom.read_register(CDROM::REG_DATA);
    assert_eq!(response, 0x02); // Motor on bit should be set

    // Test interrupt enable write (index 1)
    cdrom.write_register(CDROM::REG_INDEX, 1);
    cdrom.write_register(CDROM::REG_INT_FLAG, 0x1F);
    assert_eq!(cdrom.interrupt_enable, 0x1F);

    // Test interrupt flag read (index 1)
    cdrom.trigger_interrupt(3);
    let int_flag = cdrom.read_register(CDROM::REG_INT_ENABLE);
    assert_eq!(int_flag & 0x1F, 0x04); // INT3 set
}

// ============================================================================
// Timing Event Tests (Issue #132 Testing Requirements)
// ============================================================================

#[test]
fn test_getstat_command_timing() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;

    // Write GetStat command
    cdrom.write_register(CDROM::REG_DATA, 0x01);

    // Command should be queued, not executed
    assert!(cdrom.response_fifo.is_empty());
    assert!(cdrom.command_to_schedule.is_some());

    // Process to schedule the command
    cdrom.process_events(&mut timing, &[]);

    // Advance time less than ACK delay (5000 cycles)
    timing.pending_ticks = 4000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Response should NOT be available yet
    assert!(cdrom.response_fifo.is_empty());

    // Advance time past ACK delay
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Now response should be available (INT3)
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04); // INT3
}

#[test]
fn test_getid_command_timing_multi_stage() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;

    // Load a disc
    cdrom.disc = Some(crate::core::cdrom::DiscImage::new_dummy());

    // Write GetID command
    cdrom.write_register(CDROM::REG_DATA, 0x1A);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay (~5000 cycles)
    timing.pending_ticks = 6000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // First response (INT3) should be available
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04); // INT3

    // Clear interrupt and response
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: Second response delay (~33000 cycles)
    timing.pending_ticks = 34000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery (scheduled with minimum delay)
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Second response (INT2) should be available with disc info
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02); // INT2

    // Response should contain disc info (8 bytes)
    assert!(cdrom.response_fifo.len() >= 8);
}

#[test]
fn test_readtoc_command_timing() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;
    cdrom.disc = Some(crate::core::cdrom::DiscImage::new_dummy());

    // Write ReadTOC command
    cdrom.write_register(CDROM::REG_DATA, 0x1E);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay
    timing.pending_ticks = 6000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT3 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: TOC read delay (~500000 cycles = ~15ms)
    timing.pending_ticks = 510000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT2 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_init_command_timing() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);

    // Write Init command
    cdrom.write_register(CDROM::REG_DATA, 0x0A);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay (Init has longer delay: ~20000 cycles)
    timing.pending_ticks = 25000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT3 should be triggered, motor should be on
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
    assert!(cdrom.status.motor_on);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: Init complete delay (~70000 cycles)
    timing.pending_ticks = 75000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT2 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
}

#[test]
fn test_interrupt_delivery_delays() {
    use super::super::timing::TimingEventManager;
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);

    // Trigger first interrupt
    cdrom.async_response_fifo.push_back(0x02);
    cdrom.schedule_async_interrupt(2, &mut timing);

    // Advance time
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // First interrupt should be delivered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    let first_time = cdrom.last_interrupt_time;

    // Clear interrupt
    cdrom.interrupt_flag = 0;

    // Try to trigger second interrupt immediately
    cdrom.async_response_fifo.push_back(0x02);
    cdrom.pending_async_interrupt = 0; // Reset for new interrupt
    cdrom.schedule_async_interrupt(2, &mut timing);

    // Should be delayed by MINIMUM_INTERRUPT_DELAY (1000 cycles)
    timing.pending_ticks = 500;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Should NOT be delivered yet (too soon)
    assert_eq!(cdrom.interrupt_flag, 0);

    // Advance past minimum delay
    timing.pending_ticks = 600;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Now should be delivered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    assert!(cdrom.last_interrupt_time > first_time);
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

// Enhanced command tests for issue #119

#[test]
fn test_setmode_stores_settings() {
    let mut cdrom = CDROM::new();

    // Test double speed mode (bit 7)
    cdrom.param_fifo.push_back(0x80);
    cdrom.execute_command(0x0E); // SetMode
    assert!(cdrom.mode.double_speed);
    assert!(!cdrom.mode.size_2340);

    // Test sector size mode (bit 5)
    cdrom.param_fifo.push_back(0x20);
    cdrom.execute_command(0x0E); // SetMode
    assert!(!cdrom.mode.double_speed);
    assert!(cdrom.mode.size_2340);

    // Test multiple flags
    cdrom.param_fifo.push_back(0xA0); // Bits 7 and 5
    cdrom.execute_command(0x0E); // SetMode
    assert!(cdrom.mode.double_speed);
    assert!(cdrom.mode.size_2340);

    // Test XA-ADPCM (bit 6)
    cdrom.param_fifo.push_back(0x40);
    cdrom.execute_command(0x0E); // SetMode
    assert!(cdrom.mode.xa_adpcm);
}

#[test]
fn test_getid_with_disc() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_getid_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_getid_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let bin_data = vec![0x00; 2352 * 5];
    std::fs::write(bin_path, &bin_data).unwrap();

    let mut cdrom = CDROM::new();
    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    // Execute GetID
    cdrom.execute_command(0x1A);

    // Should have INT3 (acknowledge)
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);

    // Should have first response (status byte)
    assert!(!cdrom.response_fifo.is_empty());
    let _status = cdrom.pop_response().unwrap();

    // Should have INT2 (complete)
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);

    // Should have disc info response (8 bytes remaining after popping first status)
    assert_eq!(cdrom.response_fifo.len(), 8); // Status + 7 more bytes (Licensed, Disc type, 0x00, S, C, E, A)

    let _status2 = cdrom.pop_response().unwrap();
    let licensed = cdrom.pop_response().unwrap();
    let disc_type = cdrom.pop_response().unwrap();

    assert_eq!(licensed, 0x00); // Licensed
    assert_eq!(disc_type, 0x20); // Audio+CDROM

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_getid_without_disc() {
    let mut cdrom = CDROM::new();

    // Execute GetID without loading a disc
    cdrom.execute_command(0x1A);

    // Should trigger error interrupt (INT5)
    assert_eq!(cdrom.interrupt_flag & 0x10, 0x10);

    // Should have error status set
    assert!(cdrom.status.id_error);
}

#[test]
fn test_readtoc_with_disc() {
    // Create unique temporary files
    let bin_file = Builder::new()
        .prefix("test_readtoc_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_readtoc_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
  TRACK 02 AUDIO
    INDEX 01 00:05:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let bin_data = vec![0x00; 2352 * 300]; // 300 sectors
    std::fs::write(bin_path, &bin_data).unwrap();

    let mut cdrom = CDROM::new();
    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    // Execute ReadTOC
    cdrom.execute_command(0x1E);

    // Should have INT3 (acknowledge)
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);

    // Should have responses
    assert!(!cdrom.response_fifo.is_empty());

    // Should have INT2 (complete)
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);

    // Files automatically cleaned up when tempfile goes out of scope
}

#[test]
fn test_readtoc_without_disc() {
    let mut cdrom = CDROM::new();

    // Execute ReadTOC without loading a disc
    cdrom.execute_command(0x1E);

    // Should trigger error interrupt (INT5)
    assert_eq!(cdrom.interrupt_flag & 0x10, 0x10);

    // Should have error status set
    assert!(cdrom.status.id_error);
}

#[test]
fn test_test_command_bios_date() {
    let mut cdrom = CDROM::new();

    // Execute Test command with sub-function 0x20 (Get BIOS date)
    cdrom.param_fifo.push_back(0x20);
    cdrom.execute_command(0x19);

    // Should have INT3 (acknowledge)
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);

    // Should have 4 bytes of response (date)
    assert_eq!(cdrom.response_fifo.len(), 4);

    let year = cdrom.pop_response().unwrap();
    let month = cdrom.pop_response().unwrap();
    let day = cdrom.pop_response().unwrap();
    let version = cdrom.pop_response().unwrap();

    assert_eq!(year, 0x98); // 1998
    assert_eq!(month, 0x08); // August
    assert_eq!(day, 0x07); // 7th
    assert_eq!(version, 0xC3); // Version byte
}

#[test]
fn test_test_command_chip_id() {
    let mut cdrom = CDROM::new();

    // Execute Test command with sub-function 0x04 (Get chip ID)
    cdrom.param_fifo.push_back(0x04);
    cdrom.execute_command(0x19);

    // Should have INT3 (acknowledge)
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);

    // Should have 5 bytes of response
    assert_eq!(cdrom.response_fifo.len(), 5);
}

#[test]
fn test_test_command_no_params() {
    let mut cdrom = CDROM::new();

    // Execute Test command without parameters
    cdrom.execute_command(0x19);

    // Should trigger error interrupt (INT5)
    assert_eq!(cdrom.interrupt_flag & 0x10, 0x10);
}

#[test]
fn test_test_command_unknown_subfunction() {
    let mut cdrom = CDROM::new();

    // Execute Test command with unknown sub-function
    cdrom.param_fifo.push_back(0xFF);
    cdrom.execute_command(0x19);

    // Should have INT3 (acknowledge)
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);

    // Should have status byte response
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_bios_boot_sequence() {
    // Test a typical BIOS boot sequence with CD-ROM commands
    let bin_file = Builder::new()
        .prefix("test_bios_seq_")
        .suffix(".bin")
        .tempfile()
        .unwrap();
    let bin_path = bin_file.path();
    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();

    let cue_file = Builder::new()
        .prefix("test_bios_seq_")
        .suffix(".cue")
        .tempfile()
        .unwrap();
    let cue_path = cue_file.path();

    let cue_content = format!(
        r#"FILE "{}" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:02:00
"#,
        bin_name
    );
    std::fs::write(cue_path, cue_content).unwrap();

    let bin_data = vec![0x00; 2352 * 100];
    std::fs::write(bin_path, &bin_data).unwrap();

    let mut cdrom = CDROM::new();
    cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

    // 1. Init
    cdrom.execute_command(0x0A);
    assert!(cdrom.status.motor_on);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // 2. SetMode
    cdrom.param_fifo.push_back(0x00); // Normal mode
    cdrom.execute_command(0x0E);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // 3. ReadTOC
    cdrom.execute_command(0x1E);
    assert_ne!(cdrom.interrupt_flag, 0);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // 4. GetID
    cdrom.execute_command(0x1A);
    assert_ne!(cdrom.interrupt_flag, 0);
    assert!(!cdrom.response_fifo.is_empty());

    // Files automatically cleaned up when tempfile goes out of scope
}
