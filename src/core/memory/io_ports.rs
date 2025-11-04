// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! I/O Port Operations Module
//!
//! This module handles memory-mapped I/O port operations for the PlayStation memory bus.
//! It implements read and write operations for various hardware components including:
//!
//! - **GPU**: Graphics Processing Unit registers (GP0, GP1, GPUREAD, GPUSTAT)
//! - **Controller**: Joypad and memory card interface registers
//! - **Timers**: Three root counter/timer channels (0-2)
//! - **CD-ROM**: CD-ROM drive control and data registers
//! - **Interrupts**: Interrupt controller status and mask registers
//!
//! All I/O port operations are handled through 32-bit and 8-bit read/write methods
//! that route to the appropriate hardware component based on the physical address.

use super::Bus;
use crate::core::error::Result;

impl Bus {
    /// Read from I/O port (32-bit)
    ///
    /// Handles reads from memory-mapped I/O registers including GPU registers.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    ///
    /// # Returns
    ///
    /// The 32-bit value read from the I/O port
    pub(super) fn read_io_port32(&self, paddr: u32) -> Result<u32> {
        match paddr {
            // GPU GPUREAD register (0x1F801810)
            Self::GPU_GP0 => {
                if let Some(gpu) = &self.gpu {
                    let value = gpu.borrow_mut().read_gpuread();
                    log::trace!("GPUREAD (0x{:08X}) -> 0x{:08X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("GPUREAD access before GPU initialized");
                    Ok(0)
                }
            }

            // GPU GPUSTAT register (0x1F801814)
            Self::GPU_GP1 => {
                if let Some(gpu) = &self.gpu {
                    let value = gpu.borrow().status();
                    log::trace!("GPUSTAT (0x{:08X}) -> 0x{:08X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("GPUSTAT access before GPU initialized");
                    Ok(0)
                }
            }

            // Controller JOY_RX_DATA register (0x1F801040)
            Self::JOY_DATA => {
                if let Some(controller_ports) = &self.controller_ports {
                    let value = controller_ports.borrow_mut().read_rx_data() as u32;
                    log::trace!("JOY_RX_DATA read at 0x{:08X} -> 0x{:02X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("JOY_RX_DATA access before controller_ports initialized");
                    Ok(0xFF)
                }
            }

            // Controller JOY_STAT register (0x1F801044)
            Self::JOY_STAT => {
                if let Some(controller_ports) = &self.controller_ports {
                    let value = controller_ports.borrow().read_stat();
                    log::trace!("JOY_STAT read at 0x{:08X} -> 0x{:08X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("JOY_STAT access before controller_ports initialized");
                    Ok(0x05) // TX ready, RX ready
                }
            }

            // Controller JOY_MODE register (0x1F801048)
            Self::JOY_MODE => {
                if let Some(controller_ports) = &self.controller_ports {
                    let value = controller_ports.borrow().read_mode() as u32;
                    log::trace!("JOY_MODE read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("JOY_MODE access before controller_ports initialized");
                    Ok(0x000D)
                }
            }

            // Controller JOY_CTRL register (0x1F80104A)
            Self::JOY_CTRL => {
                if let Some(controller_ports) = &self.controller_ports {
                    let value = controller_ports.borrow().read_ctrl() as u32;
                    log::trace!("JOY_CTRL read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("JOY_CTRL access before controller_ports initialized");
                    Ok(0)
                }
            }

            // Controller JOY_BAUD register (0x1F80104E)
            Self::JOY_BAUD => {
                if let Some(controller_ports) = &self.controller_ports {
                    let value = controller_ports.borrow().read_baud() as u32;
                    log::trace!("JOY_BAUD read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("JOY_BAUD access before controller_ports initialized");
                    Ok(0)
                }
            }

            // Interrupt Status register (I_STAT) (0x1F801070)
            Self::I_STAT => {
                if let Some(interrupt_controller) = &self.interrupt_controller {
                    let value = interrupt_controller.borrow().read_status();
                    log::trace!("I_STAT read at 0x{:08X} -> 0x{:08X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("I_STAT access before interrupt_controller initialized");
                    Ok(0)
                }
            }

            // Interrupt Mask register (I_MASK) (0x1F801074)
            Self::I_MASK => {
                if let Some(interrupt_controller) = &self.interrupt_controller {
                    let value = interrupt_controller.borrow().read_mask();
                    log::trace!("I_MASK read at 0x{:08X} -> 0x{:08X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("I_MASK access before interrupt_controller initialized");
                    Ok(0)
                }
            }

            // Timer 0 Counter (0x1F801100)
            Self::TIMER0_COUNTER => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(0).read_counter() as u32;
                    log::trace!("TIMER0_COUNTER read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER0_COUNTER access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 0 Mode (0x1F801104)
            Self::TIMER0_MODE => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow_mut().channel_mut(0).read_mode() as u32;
                    log::trace!("TIMER0_MODE read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER0_MODE access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 0 Target (0x1F801108)
            Self::TIMER0_TARGET => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(0).read_target() as u32;
                    log::trace!("TIMER0_TARGET read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER0_TARGET access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 1 Counter (0x1F801110)
            Self::TIMER1_COUNTER => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(1).read_counter() as u32;
                    log::trace!("TIMER1_COUNTER read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER1_COUNTER access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 1 Mode (0x1F801114)
            Self::TIMER1_MODE => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow_mut().channel_mut(1).read_mode() as u32;
                    log::trace!("TIMER1_MODE read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER1_MODE access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 1 Target (0x1F801118)
            Self::TIMER1_TARGET => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(1).read_target() as u32;
                    log::trace!("TIMER1_TARGET read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER1_TARGET access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 2 Counter (0x1F801120)
            Self::TIMER2_COUNTER => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(2).read_counter() as u32;
                    log::trace!("TIMER2_COUNTER read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER2_COUNTER access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 2 Mode (0x1F801124)
            Self::TIMER2_MODE => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow_mut().channel_mut(2).read_mode() as u32;
                    log::trace!("TIMER2_MODE read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER2_MODE access before timers initialized");
                    Ok(0)
                }
            }

            // Timer 2 Target (0x1F801128)
            Self::TIMER2_TARGET => {
                if let Some(timers) = &self.timers {
                    let value = timers.borrow().channel(2).read_target() as u32;
                    log::trace!("TIMER2_TARGET read at 0x{:08X} -> 0x{:04X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("TIMER2_TARGET access before timers initialized");
                    Ok(0)
                }
            }

            // Other I/O ports (stub for now)
            _ => {
                log::info!("I/O port read at 0x{:08X}", paddr);
                Ok(0)
            }
        }
    }

    /// Write to I/O port (32-bit)
    ///
    /// Handles writes to memory-mapped I/O registers including GPU registers.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    /// * `value` - Value to write
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub(super) fn write_io_port32(&mut self, paddr: u32, value: u32) -> Result<()> {
        match paddr {
            // GPU GP0 register (0x1F801810) - commands and data
            Self::GPU_GP0 => {
                log::info!("GP0 write = 0x{:08X}", value);
                if let Some(gpu) = &self.gpu {
                    gpu.borrow_mut().write_gp0(value);
                    Ok(())
                } else {
                    log::warn!("GP0 write before GPU initialized");
                    Ok(())
                }
            }

            // GPU GP1 register (0x1F801814) - control commands
            Self::GPU_GP1 => {
                log::info!("GP1 write = 0x{:08X}", value);
                if let Some(gpu) = &self.gpu {
                    gpu.borrow_mut().write_gp1(value);
                    Ok(())
                } else {
                    log::warn!("GP1 write before GPU initialized");
                    Ok(())
                }
            }

            // Controller JOY_TX_DATA register (0x1F801040)
            Self::JOY_DATA => {
                if let Some(controller_ports) = &self.controller_ports {
                    controller_ports.borrow_mut().write_tx_data(value as u8);
                    log::trace!("JOY_TX_DATA write at 0x{:08X} = 0x{:02X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("JOY_TX_DATA write before controller_ports initialized");
                    Ok(())
                }
            }

            // Controller JOY_MODE register (0x1F801048)
            Self::JOY_MODE => {
                if let Some(controller_ports) = &self.controller_ports {
                    controller_ports.borrow_mut().write_mode(value as u16);
                    log::trace!("JOY_MODE write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("JOY_MODE write before controller_ports initialized");
                    Ok(())
                }
            }

            // Controller JOY_CTRL register (0x1F80104A)
            Self::JOY_CTRL => {
                if let Some(controller_ports) = &self.controller_ports {
                    controller_ports.borrow_mut().write_ctrl(value as u16);
                    log::trace!("JOY_CTRL write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("JOY_CTRL write before controller_ports initialized");
                    Ok(())
                }
            }

            // Controller JOY_BAUD register (0x1F80104E)
            Self::JOY_BAUD => {
                if let Some(controller_ports) = &self.controller_ports {
                    controller_ports.borrow_mut().write_baud(value as u16);
                    log::trace!("JOY_BAUD write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("JOY_BAUD write before controller_ports initialized");
                    Ok(())
                }
            }

            // Interrupt Status register (I_STAT) (0x1F801070)
            Self::I_STAT => {
                if let Some(interrupt_controller) = &self.interrupt_controller {
                    interrupt_controller.borrow_mut().write_status(value);
                    log::trace!("I_STAT write at 0x{:08X} = 0x{:08X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("I_STAT write before interrupt_controller initialized");
                    Ok(())
                }
            }

            // Interrupt Mask register (I_MASK) (0x1F801074)
            Self::I_MASK => {
                if let Some(interrupt_controller) = &self.interrupt_controller {
                    interrupt_controller.borrow_mut().write_mask(value);
                    log::trace!("I_MASK write at 0x{:08X} = 0x{:08X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("I_MASK write before interrupt_controller initialized");
                    Ok(())
                }
            }

            // Timer 0 Counter (0x1F801100)
            Self::TIMER0_COUNTER => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(0)
                        .write_counter(value as u16);
                    log::trace!("TIMER0_COUNTER write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER0_COUNTER write before timers initialized");
                    Ok(())
                }
            }

            // Timer 0 Mode (0x1F801104)
            Self::TIMER0_MODE => {
                if let Some(timers) = &self.timers {
                    timers.borrow_mut().channel_mut(0).write_mode(value as u16);
                    log::trace!("TIMER0_MODE write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER0_MODE write before timers initialized");
                    Ok(())
                }
            }

            // Timer 0 Target (0x1F801108)
            Self::TIMER0_TARGET => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(0)
                        .write_target(value as u16);
                    log::trace!("TIMER0_TARGET write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER0_TARGET write before timers initialized");
                    Ok(())
                }
            }

            // Timer 1 Counter (0x1F801110)
            Self::TIMER1_COUNTER => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(1)
                        .write_counter(value as u16);
                    log::trace!("TIMER1_COUNTER write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER1_COUNTER write before timers initialized");
                    Ok(())
                }
            }

            // Timer 1 Mode (0x1F801114)
            Self::TIMER1_MODE => {
                if let Some(timers) = &self.timers {
                    timers.borrow_mut().channel_mut(1).write_mode(value as u16);
                    log::trace!("TIMER1_MODE write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER1_MODE write before timers initialized");
                    Ok(())
                }
            }

            // Timer 1 Target (0x1F801118)
            Self::TIMER1_TARGET => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(1)
                        .write_target(value as u16);
                    log::trace!("TIMER1_TARGET write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER1_TARGET write before timers initialized");
                    Ok(())
                }
            }

            // Timer 2 Counter (0x1F801120)
            Self::TIMER2_COUNTER => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(2)
                        .write_counter(value as u16);
                    log::trace!("TIMER2_COUNTER write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER2_COUNTER write before timers initialized");
                    Ok(())
                }
            }

            // Timer 2 Mode (0x1F801124)
            Self::TIMER2_MODE => {
                if let Some(timers) = &self.timers {
                    timers.borrow_mut().channel_mut(2).write_mode(value as u16);
                    log::trace!("TIMER2_MODE write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER2_MODE write before timers initialized");
                    Ok(())
                }
            }

            // Timer 2 Target (0x1F801128)
            Self::TIMER2_TARGET => {
                if let Some(timers) = &self.timers {
                    timers
                        .borrow_mut()
                        .channel_mut(2)
                        .write_target(value as u16);
                    log::trace!("TIMER2_TARGET write at 0x{:08X} = 0x{:04X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("TIMER2_TARGET write before timers initialized");
                    Ok(())
                }
            }

            // Other I/O ports (stub for now)
            _ => {
                log::info!("I/O port write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())
            }
        }
    }

    /// Read from I/O port (8-bit)
    ///
    /// Handles reads from 8-bit memory-mapped I/O registers including CD-ROM registers.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    ///
    /// # Returns
    ///
    /// The 8-bit value read from the I/O port
    pub(super) fn read_io_port8(&self, paddr: u32) -> Result<u8> {
        match paddr {
            // CD-ROM Index/Status register (0x1F801800)
            // Read: Status register with FIFO states and busy flags
            Self::CDROM_INDEX => {
                if let Some(cdrom) = &self.cdrom {
                    let value = cdrom.borrow().read_status();
                    log::trace!("CDROM_STATUS read at 0x{:08X} -> 0x{:02X}", paddr, value);
                    Ok(value)
                } else {
                    log::warn!("CDROM_STATUS access before CDROM initialized");
                    // Return "ready" status (0x18): Parameter FIFO empty and not full
                    Ok(0x18)
                }
            }

            // CD-ROM data register (0x1F801801)
            Self::CDROM_REG1 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    let value = match index {
                        0 => {
                            // Response FIFO
                            cdrom.borrow_mut().pop_response().unwrap_or(0)
                        }
                        1 => {
                            // Response FIFO (same as index 0)
                            cdrom.borrow_mut().pop_response().unwrap_or(0)
                        }
                        2 => {
                            // Response FIFO (same as index 0)
                            cdrom.borrow_mut().pop_response().unwrap_or(0)
                        }
                        3 => {
                            // Response FIFO (same as index 0)
                            cdrom.borrow_mut().pop_response().unwrap_or(0)
                        }
                        _ => 0,
                    };
                    log::trace!(
                        "CDROM_REG1 (index {}) read at 0x{:08X} -> 0x{:02X}",
                        index,
                        paddr,
                        value
                    );
                    Ok(value)
                } else {
                    log::warn!("CDROM_REG1 access before CDROM initialized");
                    Ok(0)
                }
            }

            // CD-ROM interrupt flag register (0x1F801802)
            Self::CDROM_REG2 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    let value = match index {
                        0 | 2 => {
                            // Interrupt flag
                            cdrom.borrow().interrupt_flag()
                        }
                        1 | 3 => {
                            // Interrupt enable
                            cdrom.borrow().interrupt_enable()
                        }
                        _ => 0,
                    };
                    log::trace!(
                        "CDROM_REG2 (index {}) read at 0x{:08X} -> 0x{:02X}",
                        index,
                        paddr,
                        value
                    );
                    Ok(value)
                } else {
                    log::warn!("CDROM_REG2 access before CDROM initialized");
                    Ok(0)
                }
            }

            // CD-ROM interrupt enable register (0x1F801803)
            Self::CDROM_REG3 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    let value = match index {
                        0 | 2 => {
                            // Interrupt enable
                            cdrom.borrow().interrupt_enable()
                        }
                        1 | 3 => {
                            // Interrupt flag
                            cdrom.borrow().interrupt_flag()
                        }
                        _ => 0,
                    };
                    log::trace!(
                        "CDROM_REG3 (index {}) read at 0x{:08X} -> 0x{:02X}",
                        index,
                        paddr,
                        value
                    );
                    Ok(value)
                } else {
                    log::warn!("CDROM_REG3 access before CDROM initialized");
                    Ok(0)
                }
            }

            // Other I/O ports (stub for now)
            _ => {
                log::trace!("I/O port read8 at 0x{:08X}", paddr);
                Ok(0)
            }
        }
    }

    /// Write to I/O port (8-bit)
    ///
    /// Handles writes to 8-bit memory-mapped I/O registers including CD-ROM registers.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    /// * `value` - Value to write
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub(super) fn write_io_port8(&mut self, paddr: u32, value: u8) -> Result<()> {
        match paddr {
            // CD-ROM Index/Status register (0x1F801800)
            Self::CDROM_INDEX => {
                if let Some(cdrom) = &self.cdrom {
                    cdrom.borrow_mut().set_index(value);
                    log::trace!("CDROM_INDEX write at 0x{:08X} = 0x{:02X}", paddr, value);
                    Ok(())
                } else {
                    log::warn!("CDROM_INDEX write before CDROM initialized");
                    Ok(())
                }
            }

            // CD-ROM command/parameter register (0x1F801801)
            Self::CDROM_REG1 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    match index {
                        0 => {
                            // Command register
                            log::debug!("CDROM command 0x{:02X} at 0x{:08X}", value, paddr);
                            cdrom.borrow_mut().execute_command(value);
                        }
                        1..=3 => {
                            // Parameter FIFO (same for all other indices)
                            log::trace!(
                                "CDROM_REG1 (index {}) parameter write at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                            cdrom.borrow_mut().push_param(value);
                        }
                        _ => {}
                    }
                    Ok(())
                } else {
                    log::warn!("CDROM_REG1 write before CDROM initialized");
                    Ok(())
                }
            }

            // CD-ROM interrupt acknowledge register (0x1F801802)
            Self::CDROM_REG2 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    match index {
                        0 | 2 => {
                            // Interrupt acknowledge
                            log::trace!(
                                "CDROM_REG2 (index {}) interrupt ack at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                            cdrom.borrow_mut().acknowledge_interrupt(value);
                        }
                        1 | 3 => {
                            // Interrupt enable
                            log::trace!(
                                "CDROM_REG2 (index {}) interrupt enable at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                            cdrom.borrow_mut().set_interrupt_enable(value);
                        }
                        _ => {}
                    }
                    Ok(())
                } else {
                    log::warn!("CDROM_REG2 write before CDROM initialized");
                    Ok(())
                }
            }

            // CD-ROM control register (0x1F801803)
            Self::CDROM_REG3 => {
                if let Some(cdrom) = &self.cdrom {
                    let index = cdrom.borrow().index();
                    match index {
                        0 => {
                            // Request register (not yet implemented)
                            log::trace!(
                                "CDROM_REG3 (index {}) request write at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                        }
                        1 => {
                            // Interrupt enable
                            log::trace!(
                                "CDROM_REG3 (index {}) interrupt enable at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                            cdrom.borrow_mut().set_interrupt_enable(value);
                        }
                        2 => {
                            // Audio volume for left CD output to left SPU
                            log::trace!(
                                "CDROM_REG3 (index {}) audio vol L->L at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                        }
                        3 => {
                            // Audio volume for right CD output to right SPU
                            log::trace!(
                                "CDROM_REG3 (index {}) audio vol R->R at 0x{:08X} = 0x{:02X}",
                                index,
                                paddr,
                                value
                            );
                        }
                        _ => {}
                    }
                    Ok(())
                } else {
                    log::warn!("CDROM_REG3 write before CDROM initialized");
                    Ok(())
                }
            }

            // Other I/O ports (stub for now)
            _ => {
                log::trace!("I/O port write8 at 0x{:08X} = 0x{:02X}", paddr, value);
                Ok(())
            }
        }
    }
}
