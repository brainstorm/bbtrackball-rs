// Borrowed and modified from
// https://github.com/TeXitoi/keyberon/blob/master/src/keyboard.rs

use crate::hid::{HidDevice, Protocol, ReportType, Subclass};
// use crate::key_code::KbHidReport;

// Stolen from the working example at https://stackoverflow.com/questions/14904009/simple-joystick-hid-report-descriptor-doesnt-work
const REPORT_DESCRIPTOR: &[u8] = &[
    0x0E, 0x0C, 0xCA,  // Unknown (bTag: 0x00, bType: 0x03)
    0x48,              // Designator Minimum
    0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
    0x09, 0x04,        // Usage (Joystick)
    0xA1, 0x01,        // Collection (Application)
    0x15, 0x81,        //   Logical Minimum (-127)
    0x25, 0x7F,        //   Logical Maximum (127)
    0x05, 0x01,        //   Usage Page (Generic Desktop Ctrls)
    0x09, 0x01,        //   Usage (Pointer)
    0xA1, 0x00,        //   Collection (Physical)
    0x09, 0x30,        //     Usage (X)
    0x09, 0x31,        //     Usage (Y)
    0x75, 0x08,        //     Report Size (8)
    0x95, 0x02,        //     Report Count (2)
    0x81, 0x02,        //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0,              //   End Collection
    0x05, 0x09,        //   Usage Page (Button)
    0x19, 0x01,        //   Usage Minimum (0x01)
    0x29, 0x1f,        //   Usage Maximum
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x1f,        //   Report Count
    0x55, 0x00,        //   Unit Exponent (0)
    0x65, 0x00,        //   Unit (None)
    0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0,              // End Collection
];


#[derive(Clone)]
pub struct TrackballReport {
    pub x: i8,
    pub y: i8,
    pub buttons: [u8; 3],
}

impl TrackballReport {
    pub fn new(x: i8, y: i8, buttons: [u8; 3]) -> Self {
        Self {x, y, buttons}
    }
    pub fn as_bytes(&self) -> [u8; 5] {
        [self.x as u8, self.y as u8, self.buttons[0], self.buttons[1], self.buttons[2]]
    }
}

impl Default for TrackballReport {
    fn default() -> Self {
        Self {
            x: 50,
            y: 50,
            buttons: [0; 3],
        }
    }
}



pub struct Trackball {
    report: [u8; 5],
}
impl Trackball {
    pub fn new() -> Self {
        Self {
            report: TrackballReport::default().as_bytes(),
        }
    }
    pub fn set_report(&mut self, report: TrackballReport) {
        self.report = report.as_bytes();
    }
}

impl HidDevice for Trackball {
    fn subclass(&self) -> Subclass {
        Subclass::None
    }

    fn protocol(&self) -> Protocol {
        Protocol::None
    }

    fn report_descriptor(&self) -> &[u8] {
        REPORT_DESCRIPTOR
    }

    fn get_report(&mut self, report_type: ReportType, _report_id: u8) -> Result<&[u8], ()> {
        match report_type {
            ReportType::Input => Ok(&self.report),
            _ => Err(()),
        }
    }

    fn set_report(
        &mut self,
        _report_type: ReportType,
        _report_id: u8,
        _data: &[u8],
    ) -> Result<(), ()> {
        Ok(())
    }
}
