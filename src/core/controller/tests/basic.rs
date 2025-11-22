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

use super::super::*;

#[test]
fn test_controller_initialization() {
    let controller = Controller::new();

    // All buttons should be released initially (0xFFFF - active low)
    assert_eq!(controller.get_buttons(), 0xFFFF);
    assert_eq!(controller.state, SerialState::Idle);
}
