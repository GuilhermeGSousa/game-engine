pub fn get_terminal_size() -> (u32, u32) {
    platform::get().unwrap_or((80, 24))
}

#[cfg(unix)]
mod platform {
    use std::mem::MaybeUninit;

    pub fn get() -> Option<(u32, u32)> {
        #[repr(C)]
        struct Winsize {
            ws_row: u16,
            ws_col: u16,
            _ws_xpixel: u16,
            _ws_ypixel: u16,
        }

        let mut ws = MaybeUninit::<Winsize>::uninit();
        let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, ws.as_mut_ptr()) };
        if ret != 0 {
            return None;
        }
        let ws = unsafe { ws.assume_init() };
        if ws.ws_col == 0 || ws.ws_row == 0 {
            return None;
        }
        Some((ws.ws_col as u32, ws.ws_row as u32))
    }
}

#[cfg(not(unix))]
mod platform {
    pub fn get() -> Option<(u32, u32)> {
        None
    }
}
