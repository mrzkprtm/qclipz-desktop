// Small Win32 / DXGI helpers, declared by hand to avoid pulling in the `windows` crate.
use std::ffi::c_void;

pub type Hwnd = isize;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(cb: unsafe extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
    fn IsWindowVisible(h: Hwnd) -> i32;
    fn IsIconic(h: Hwnd) -> i32;
    fn GetWindowTextW(h: Hwnd, buf: *mut u16, max: i32) -> i32;
    fn GetWindowLongPtrW(h: Hwnd, idx: i32) -> isize;
    fn GetWindow(h: Hwnd, cmd: u32) -> Hwnd;
    fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
    fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
    fn SetForegroundWindow(h: Hwnd) -> i32;
    fn IsWindow(h: Hwnd) -> i32;
    fn SetWindowDisplayAffinity(h: Hwnd, affinity: u32) -> i32;
}
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(h: Hwnd, attr: u32, out: *mut c_void, size: u32) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn CloseHandle(h: isize) -> i32;
    fn QueryFullProcessImageNameW(h: isize, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
}
#[link(name = "dxgi")]
unsafe extern "system" {
    fn CreateDXGIFactory1(riid: *const Guid, out: *mut *mut c_void) -> i32;
}

const GWL_EXSTYLE: i32 = -20;
const WS_EX_TOOLWINDOW: isize = 0x80;
const GW_OWNER: u32 = 4;
const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
const DWMWA_CLOAKED: u32 = 14;
const SW_RESTORE: i32 = 9;
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

pub struct WindowEntry {
    pub hwnd: Hwnd,
    pub title: String,
    pub exe: String,
    pub minimized: bool,
}

fn exe_name(pid: u32) -> String {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h == 0 {
            return String::new();
        }
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().unwrap_or_default().to_string()
    }
}

unsafe extern "system" fn collect(h: Hwnd, lparam: isize) -> i32 {
    let out = unsafe { &mut *(lparam as *mut Vec<WindowEntry>) };
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, 4);
        if IsWindowVisible(h) == 0
            || cloaked != 0
            || GetWindow(h, GW_OWNER) != 0
            || GetWindowLongPtrW(h, GWL_EXSTYLE) & WS_EX_TOOLWINDOW != 0
            || pid == std::process::id()
        {
            return 1;
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
        if n > 0 {
            let title = String::from_utf16_lossy(&buf[..n as usize]);
            if title != "Program Manager" {
                out.push(WindowEntry { hwnd: h, title, exe: exe_name(pid), minimized: IsIconic(h) != 0 });
            }
        }
    }
    1
}

// Top-level app windows, roughly the Alt+Tab list.
pub fn list_windows() -> Vec<WindowEntry> {
    let mut out = Vec::new();
    unsafe { EnumWindows(collect, &mut out as *mut _ as isize) };
    out
}

// Restore if minimized and bring to the front, so it's visible for capture.
pub fn bring_to_front(h: Hwnd) -> Result<(), String> {
    unsafe {
        if IsWindow(h) == 0 {
            return Err("that window was closed — refresh the list".into());
        }
        if IsIconic(h) != 0 {
            ShowWindow(h, SW_RESTORE);
        }
        SetForegroundWindow(h);
    }
    Ok(())
}

// Visible bounds of a window in physical px (without the invisible resize border).
pub fn window_bounds(h: Hwnd) -> Option<Rect> {
    let mut r = Rect::default();
    let hr = unsafe {
        DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut Rect as *mut c_void, 16)
    };
    (hr == 0 && r.right > r.left && r.bottom > r.top).then_some(r)
}

// Hide a window from screen capture (Windows 10 2004+). Ignored on older systems.
pub fn exclude_from_capture(h: Hwnd) {
    unsafe { SetWindowDisplayAffinity(h, WDA_EXCLUDEFROMCAPTURE) };
}

#[repr(C)]
pub struct Guid(u32, u16, u16, [u8; 8]);
const IID_IDXGI_FACTORY1: Guid = Guid(0x770aae78, 0xf26f, 0x4dba, [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87]);

#[repr(C)]
struct OutputDesc {
    device_name: [u16; 32],
    desktop: Rect,
    attached: i32,
    rotation: u32,
    monitor: isize,
}

// Nth entry of a COM object's vtable.
unsafe fn vfn(obj: *mut c_void, idx: usize) -> *const c_void {
    unsafe { *(*(obj as *const *const *const c_void)).add(idx) }
}
unsafe fn release(obj: *mut c_void) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(vfn(obj, 2));
        f(obj);
    }
}

// Desktop rects of the default GPU's outputs, in the order ffmpeg's
// ddagrab `output_idx` uses (IDXGIAdapter::EnumOutputs on adapter 0).
pub fn dxgi_outputs() -> Vec<Rect> {
    let mut rects = Vec::new();
    unsafe {
        let mut factory: *mut c_void = std::ptr::null_mut();
        if CreateDXGIFactory1(&IID_IDXGI_FACTORY1, &mut factory) < 0 || factory.is_null() {
            return rects;
        }
        // IDXGIFactory1::EnumAdapters1 = vtable slot 12
        let enum_adapters: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32 =
            std::mem::transmute(vfn(factory, 12));
        let mut adapter: *mut c_void = std::ptr::null_mut();
        if enum_adapters(factory, 0, &mut adapter) >= 0 && !adapter.is_null() {
            // IDXGIAdapter::EnumOutputs = slot 7
            let enum_outputs: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32 =
                std::mem::transmute(vfn(adapter, 7));
            let mut i = 0;
            loop {
                let mut output: *mut c_void = std::ptr::null_mut();
                if enum_outputs(adapter, i, &mut output) < 0 || output.is_null() {
                    break;
                }
                // IDXGIOutput::GetDesc = slot 7
                let get_desc: unsafe extern "system" fn(*mut c_void, *mut OutputDesc) -> i32 =
                    std::mem::transmute(vfn(output, 7));
                let mut desc: OutputDesc = std::mem::zeroed();
                if get_desc(output, &mut desc) >= 0 {
                    rects.push(desc.desktop);
                }
                release(output);
                i += 1;
            }
            release(adapter);
        }
        release(factory);
    }
    rects
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHFileOperationW(op: *mut FileOp) -> i32;
}

#[repr(C)]
struct FileOp {
    hwnd: Hwnd,
    func: u32,
    from: *const u16,
    to: *const u16,
    flags: u16,
    aborted: i32,
    mappings: *mut c_void,
    title: *const u16,
}

// Move a file to the Recycle Bin instead of deleting it for good.
pub fn recycle(path: &std::path::Path) -> Result<(), String> {
    const FO_DELETE: u32 = 3;
    const FOF_SILENT: u16 = 0x4;
    const FOF_NOCONFIRMATION: u16 = 0x10;
    const FOF_ALLOWUNDO: u16 = 0x40;
    const FOF_NOERRORUI: u16 = 0x400;
    use std::os::windows::ffi::OsStrExt;
    // pFrom is a list of paths ending in a double NUL.
    let from: Vec<u16> = path.as_os_str().encode_wide().chain([0, 0]).collect();
    let mut op = FileOp {
        hwnd: 0,
        func: FO_DELETE,
        from: from.as_ptr(),
        to: std::ptr::null(),
        flags: FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI,
        aborted: 0,
        mappings: std::ptr::null_mut(),
        title: std::ptr::null(),
    };
    match unsafe { SHFileOperationW(&mut op) } {
        0 if op.aborted == 0 => Ok(()),
        code => Err(format!("couldn't move {} to the Recycle Bin (error {code})", path.display())),
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
}

// Re-assert topmost so the window sits above the (also topmost) taskbar.
pub fn raise_topmost(h: Hwnd) {
    const HWND_TOPMOST: Hwnd = -1;
    const SWP_NOSIZE: u32 = 0x1;
    const SWP_NOMOVE: u32 = 0x2;
    const SWP_SHOWWINDOW: u32 = 0x40;
    unsafe { SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW) };
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetPriorityClass(process: isize, class: u32) -> i32;
}

// Run a child process below normal priority, so the apps being recorded (a game, a
// video) keep their smoothness when the CPU is busy; dropping a capture frame is better
// than the source itself stuttering, which would end up in the recording anyway.
pub fn lower_priority(child: &std::process::Child) {
    use std::os::windows::io::AsRawHandle;
    const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x4000;
    unsafe { SetPriorityClass(child.as_raw_handle() as isize, BELOW_NORMAL_PRIORITY_CLASS) };
}
