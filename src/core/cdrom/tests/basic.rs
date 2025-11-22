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

//! Basic CDROM functionality tests (initialization, reset, registers)

use super::super::*;

#[test]
fn test_cdrom_initialization() {
    let cdrom = CDROM::new();
    assert_eq!(cdrom.state, CDState::Idle);
    assert_eq!(cdrom.position.minute, 0);
    assert_eq!(cdrom.position.second, 2);
    assert_eq!(cdrom.position.sector, 0);
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
    use crate::core::timing::TimingEventManager;
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
