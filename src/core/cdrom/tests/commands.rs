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

//! CDROM command processing tests

use super::super::*;
use tempfile::Builder;

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
