use crate::pio::Pio;

pub fn get_time() -> u64 {
    Rtc::new().time()
}

fn cvt_bcd(value: usize) -> usize {
    (value & 0xF) + ((value / 16) * 10)
}

pub struct Rtc {
    addr: Pio<u8>,
    data: Pio<u8>,
    nmi: bool,
}

impl Rtc {
    pub fn new() -> Self {
        Rtc {
            addr: Pio::new(0x70),
            data: Pio::new(0x71),
            nmi: false,
        }
    }

    unsafe fn read(&mut self, reg: u8) -> u8 {
        if self.nmi {
            self.addr.write(reg & 0x7F);
        } else {
            self.addr.write(reg | 0x80);
        }
        self.data.read()
    }

    unsafe fn wait(&mut self, full: bool) {
        if full {
            while self.read(0xA) & 0x80 != 0x80 {}
        }
        while self.read(0xA) & 0x80 == 0x80 {}
    }

    pub unsafe fn time_no_wait(&mut self) -> u64 {
        let mut second = self.read(0) as usize;
        let mut minute = self.read(2) as usize;
        let mut hour = self.read(4) as usize;
        let mut day = self.read(7) as usize;
        let mut month = self.read(8) as usize;
        let mut year = self.read(9) as usize;
        let mut century = 20;
        let register_b = self.read(0xB);

        if register_b & 4 != 4 {
            second = cvt_bcd(second);
            minute = cvt_bcd(minute);
            hour = cvt_bcd(hour & 0x7F) | (hour & 0x80);
            day = cvt_bcd(day);
            month = cvt_bcd(month);
            year = cvt_bcd(year);
        }

        if register_b & 2 != 2 || hour & 0x80 == 0x80 {
            hour = ((hour & 0x7F) + 12) % 24;
        }

        year += century * 100;

        let mut secs: u64 = (year as u64 - 1970) * 31_536_000;
        let mut leap_days = (year as u64 - 1972) / 4 + 1;
        if year % 4 == 0 && month <= 2 {
            leap_days -= 1;
        }
        secs += leap_days * 86_400;

        match month {
            2 => secs += 2_678_400,
            3 => secs += 5_097_600,
            4 => secs += 7_776_000,
            5 => secs += 10_368_000,
            6 => secs += 13_046_400,
            7 => secs += 15_638_400,
            8 => secs += 18_316_800,
            9 => secs += 20_995_200,
            10 => secs += 23_587_200,
            11 => secs += 26_265_600,
            12 => secs += 28_857_600,
            _ => (),
        }

        secs += (day as u64 - 1) * 86_400;
        secs += hour as u64 * 3600;
        secs += minute as u64 * 60;
        secs += second as u64;
        secs
    }

    pub fn time(&mut self) -> u64 {
        loop {
            unsafe {
                self.wait(false);
                let time = self.time_no_wait();
                self.wait(false);
                let next_time = self.time_no_wait();
                if time == next_time {
                    return time;
                }
            }
        }
    }
}
