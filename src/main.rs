#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]

extern crate scan_dir;
extern crate time;
extern crate regex;
extern crate winapi;
extern crate user32;
extern crate comctl32;
extern crate kernel32;
#[macro_use]
extern crate lazy_static;
extern crate widestring;

use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::fs;
use std::path::PathBuf;

use time::PreciseTime;
use scan_dir::ScanDir;

use std::ffi::OsStr;
use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::ptr;
use std::mem;
use std::collections::HashMap;

use widestring::{WideString, WideCString};

// use winapi::{HWND, WNDCLASSW, HBRUSH, HMENU, RECT,
//     WS_VISIBLE, WS_OVERLAPPEDWINDOW, 
//     WM_CREATE, WM_QUIT, WM_DESTROY, WM_CLOSE, WM_NOTIFY, WM_SIZE, WM_COMMAND,
//     LPCWSTR, LPWSTR,
//     INITCOMMONCONTROLSEX, ICC_LISTVIEW_CLASSES,
//     WS_CHILD, WS_BORDER,
//     LVS_REPORT, LVS_EDITLABELS,
//     LVCF_WIDTH, LVCF_IDEALWIDTH, LVCF_TEXT, LVCF_ORDER, LVCF_SUBITEM,
//     NMHDR, LPNMHDR };
use winapi::*;

// use winapi::commctrl::{ LVN_GETDISPINFOW, NMLVDISPINFOW, LPNMLVDISPINFOW,
//     LVN_ODCACHEHINT, NMLVCACHEHINT, LPNMLVCACHEHINT,
//     LVN_ODFINDITEMW, NMLVFINDITEMW, LPNMLVFINDITEMW
//       };

// use winapi::minwindef::*;

// use comctl32::WC_LISTVIEW;


mod lens;
mod runner;
mod config;

use config::Config;


fn list_files_in_dir(path: &str) -> Vec<lens::DirEntry> {
    // println!("Starting glob: {:?}", path);

    let mut vec: Vec<lens::DirEntry> = Vec::new();
    
    let path = PathBuf::from(path);

    if !path.exists() {
        println!("Sorry path: {:?} does not exits", path);
        return vec;
    }


    ScanDir::all().skip_hidden(true).read(path, |iter| {
        for (path, _) in iter {
            let path = path.path();
            let meta = path.metadata().expect("Failed to read metadata");

            // *** Handle file ***
            if meta.is_file() {
                let ff = vec![lens::FileEntry {
                    name: String::from(path.file_name().unwrap().to_str().unwrap()),
                    path: String::from(path.to_str().unwrap()),
                    size: meta.len()
                }];

                let e = lens::DirEntry {
                    name: String::from(path.file_name().unwrap().to_str().unwrap()),
                    path: String::from(path.to_str().unwrap()),
                    files: ff
                };

                vec.push(e);
            }

            // *** Handle dir ***
            if meta.is_dir() {

                let mut files = Vec::new();
                
                // println!("***");

                ScanDir::files().walk(&path, |iter| {
                    for (entry, name) in iter {
                        // println!("File {:?} has full path {:?}", name, entry.path());
                        let dir = entry.path();

                        // println!("File: {:?}", &dir);

                        let dir_path = dir.to_str().expect("Failed to read path");

                        let size;

                        if dir.to_str().unwrap().len() >= 260 {
                            let strr = "\\??\\".to_owned() + dir.to_str().unwrap();
                            size = fs::metadata(&strr).expect("failed to read metadata").len();
                            // println!("Long path");
                        } else {
                            size = fs::metadata(&dir).expect("failed to read metadata").len();
                        }   
    
                        files.push(lens::FileEntry {
                            name: String::from(name),
                            path: String::from(dir_path),
                            size: size
                        });
                    }
                }).unwrap();

                let e = lens::DirEntry {
                    name: String::from(path.file_name().unwrap().to_str().unwrap()),
                    path: String::from(path.to_str().unwrap()),
                    files: files,
                };

                // if e.files.len() == 0 {
                //     println!("Dir: {:?} Count: {:?} Path: {:?} ", e.name, e.files.len(), path.to_str().unwrap());
                // }
                // println!(" ");
                vec.push(e);
            }

            // println!("Dir: {:?} Path: {:?}", meta.is_dir(), path);
        }
    }).expect("Scan dir failed!");

    // println!("glob Done");
    vec
}


fn get_all_data() -> Vec<lens::DirEntry> {

    // let paths = Config.read_config();

    // for c in paths.iter() {
    //     println!("Conf: {:?}", &c);
    // }

    let mut vec = Vec::with_capacity(10_000);

    let start = PreciseTime::now();
  
    // let mut children = Vec::new();

    // for p in paths {
    //     children.push(thread::spawn(move || {
    //         let start = PreciseTime::now();

    //         let vec = list_files_in_dir(&p);
            
    //         let end = PreciseTime::now();

    //         println!("Path {:?} entries took: {:?} ms", &p, start.to(end).num_milliseconds());
    //         vec
    //     }))
    // }

    // for c in children {
    //     vec.append(&mut c.join().expect("Failed to join thread!"));
    // }

    for i in 0..10_000 {
        vec.push(lens::DirEntry {
            name: format!("Name: {:?}", i),
            path: format!("Path {:?}", i),
            files: Vec::new()

        });
    }

    vec.sort();

    let end = PreciseTime::now();

    println!("Got {:?} entries took: {:?} ms", vec.len(), start.to(end).num_milliseconds());

    vec
}


fn to_wstring(str: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(str).encode_wide().chain(once(0)).collect();
    wide.push(0);
    wide
}

fn to_string(str: &Vec<u16>) -> String {
    let vec = str.split(|c| *c == 0).next();
    if !vec.is_none() {
       std::char::decode_utf16(vec.unwrap().iter().cloned()).map(|r| r.unwrap()).collect()
    } else {
        String::new()
    }
}


// ************** Constant HWNDS **************
lazy_static! {
    static ref ALL_DATA: RwLock<Vec<lens::DirEntry>> = RwLock::new(Vec::new());
    static ref LIST_CACHE: RwLock<Vec<u64>> = RwLock::new(Vec::new());
    // static ref LIST_FOO_CACHE: RwLock<Vec<LV_ITEMW>> = RwLock::new(Vec::new());
    static ref STRING_CACHE: RwLock<HashMap<String, Vec<u16>>> = RwLock::new(HashMap::new());
}

static mut LIST_HWND: HWND = 0 as HWND;
static mut EDIT_HWND: HWND = 0 as HWND;

const IDC_MAIN_EDIT: HMENU = 100 as HMENU;
const IDC_MAIN_LISTVIEW: HMENU = 101 as HMENU;

const MAX_TEXT_LEN: u64 = 250;

// ************ End Constant HWNDS ************

pub unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT
{
    match msg {
        WM_CREATE => {
            let mut icc = mem::zeroed::<INITCOMMONCONTROLSEX>();
            icc.dwSize = mem::size_of::<INITCOMMONCONTROLSEX>() as DWORD;
            icc.dwICC = ICC_LISTVIEW_CLASSES;
            
            if comctl32::InitCommonControlsEx(&icc) == 0 {
                println!("Failed to init component");
            }
            
            create_entry(hwnd);
            test_setup_list(hwnd);

            println!("Running WM_CREATE");
            0
        }
        WM_NOTIFY => {
            // if mem::transmute_copy(src)
            // println!("l_param: {:?}", l_param);
            // let mut is_safe = false;
            // let mut ptr: LPNMLVDISPINFOW = 0 as LPNMLVDISPINFOW;

            // let pnmhdr: LPNMHDR = l_param as LPNMHDR;
            match (*(l_param as LPNMHDR) as NMHDR).code {
                LVN_GETDISPINFOA => {
                    println!("Why the A!?");
                    0
                },
                LVN_GETDISPINFOW => {
                    // let mut plvdi: NMLVDISPINFOW = *(l_param as LPNMLVDISPINFOW);
                    // let plvdi: LPNMLVDISPINFOW = (l_param as LPNMLVDISPINFOW);
            
                    let ix: usize = (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.iItem as usize;
                    let mask = (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.mask;

                    if (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.iItem < 0 {
                        println!("Request Item with negative index:{:?}", (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.iItem);
                        return 0;
                    }

                    // Retrieve information for item at index iItem.
                    //RetrieveItem( &rndItem, plvdi->item.iItem );
                   
                    if (mask & LVIF_STATE) == 0 { 
                        // println!("They want state!");
                        (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.state = 0;
                    }

                    if (mask & LVIF_IMAGE) == 0 { 
                        // println!("They want image!");
                        (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.iImage = -1;
                    }

                    if (mask & LVIF_TEXT) == 0 {
                        let len = ALL_DATA.read().unwrap().len();
                        if ix >= len -1 {
                            println!("ix bigger then index! ix: {:?} len: {:?}", ix, len );
                            return 0;

                        }
                        let ref item = ALL_DATA.read().unwrap()[ix];

                        // println!("Ix wanted: {:?} lparam: {:?} ", ix, (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.lParam);
                        // println!("wparam: {:?} lparam: {:?} ", w_param, l_param);

                        // let text = match (*plvdi as NMLVDISPINFOW).item.iSubItem {
                        //     0 => &item.name,
                        //     1 => &item.path,
                        //     2 => "100mb", //String::from(lens::pretty_size(item.size()),
                        //     n => {
                        //         println!("Found subitem: {:?}", n);
                        //         return 0;
                        //     }
                        // };
                        let f = |key| {
                            let ref vec = STRING_CACHE.read().unwrap()[(key)];
                            let ptr = STRING_CACHE.read().unwrap()[(key)].as_ptr();
                            (ptr, vec.len())
                        };

                        let (ptr, _) = match (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.iSubItem {
                            0 => f(&item.name),
                            1 => f(&item.path),
                            2 => (to_wstring("100mb").as_ptr(), 5), //String::from(lens::pretty_size(item.size()),
                            n => {
                                println!("Found subitem: {:?}", n);
                                return 0;
                            }
                        };

                        let ref vec = STRING_CACHE.read().unwrap()[&item.name];

                        if vec.as_ptr() == ptr {

                            // println!("Got right pointer");
                        }
                        // println!("Set sub: {:?} to: {:?}", text, plvdi.item.iSubItem);

                        // let text = to_wstring(text);
                        // let v = (*plvdi as NMLVDISPINFOW).item.pszText as &mut Vec<u16>;

                        // let raw_ptr = (*plvdi as NMLVDISPINFOW).item.pszText;
                        // mem::forget(plvdi);
                        // mem::forget((*plvdi as NMLVDISPINFOW).item);

                        // println!("Old string len: {:?} first: {:?}", kernel32::lstrlenW((*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.pszText), vec[0]);

                        // let ss = std::ffi::CString::new("hello dude").unwrap();
                        if (*(l_param as *const NMLVDISPINFOW)).item.cchTextMax > 0 {
                            println!("cchTextMax is {:?}", (*(l_param as *const NMLVDISPINFOW)).item.cchTextMax  );
                        }

                        let mut tem = &mut (*(l_param as *mut NMLVDISPINFOW)).item;
                        tem.pszText = vec.as_ptr() as LPWSTR;
                        // kernel32::lstrcpyW((*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.pszText, vec.as_ptr() as LPWSTR);
                        // *&mut (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.pszText = vec.as_ptr() as LPWSTR;
                        // (*(l_param as *mut NMLVDISPINFOW)).item.pszText = vec.as_ptr() as LPWSTR;
                        (*(l_param as *mut NMLVDISPINFOW)).item.cchTextMax = vec.len() as i32;
                        mem::forget(*(l_param as *mut NMLVDISPINFOW));
                        mem::forget((*(l_param as *mut NMLVDISPINFOW)).item);
                        // (*(l_param as LPNMLVDISPINFOW) as NMLVDISPINFOW).item.lpszText = vec.as_ptr() as LPWSTR;



                        println!("New string len: {:?} vec: {:?} l_param: {:?}",
                            kernel32::lstrlenW((*(l_param as *const NMLVDISPINFOW)).item.pszText),
                            vec.as_ptr() as LPWSTR,
                            l_param);

                        // (*plvdi as NMLVDISPINFOW).item.pszText = ss.as_ptr() as LPWSTR;
                        // mem::forget(ss);

                        // (*plvdi as NMLVDISPINFOW).item.cchTextMax = len as i32;
                        // mem::forget(plvdi);

                        // println!("String len {:?}",  (*plvdi as NMLVDISPINFOW).item.cchTextMax);

                        return 0;                


                        // STRING_CACHE.write().unwrap().push(text);
                        // let t = WideCString::from_ptr_str((*plvdi as NMLVDISPINFOW).item.pszText);
                        // println!("String Before: sub: {:?} text: {:?}", (*plvdi as NMLVDISPINFOW).item.iSubItem, t.to_string());
            
                        // if ix == 0 {
                        //     // let elem: LPLVITEMW = LIST_CACHE.read().unwrap()[0 as usize] as LPLVITEMW;
                        //     // let t = WideCString::from_ptr_str(elem.pszText);
                        //     // println!("String Before: sub: {:?} text: {:?}", elem.iSubItem, t.to_string());
                 
                        //     // println!("Got item: {:?}", elem.iItem);
                        //     // println!("Got item: {:?}", elem);
                        //     println!("Dude!: {:?} sub: {:?}",
                        //         (&mut (*plvdi as NMLVDISPINFOW).item) as LPLVITEMW, 
                        //         (*plvdi as NMLVDISPINFOW).item.iSubItem);
                        //     // let addr = (&mut (*plvdi as NMLVDISPINFOW).item) as *mut LV_ITEMW;
                            
                        //     // std::ptr::write(addr, *elem);
                        //     // println!("New address: {:?}", (&mut (*plvdi as NMLVDISPINFOW).item) as LPLVITEMW);
                        // }
                                        
                    }


                    user32::DefWindowProcW(hwnd, msg, w_param, l_param)
                    
                },
                LVN_ODCACHEHINT=> {
                    // let cache_hint = l_param as LPNMLVCACHEHINT;

                    // let from = (*cache_hint as NMLVCACHEHINT).iFrom;
                    // let to = (*cache_hint as NMLVCACHEHINT).iTo;
                    // println!("Got cache hint from {:?} to: {:?}", from, to);


                    0
                },
                LVN_ODFINDITEMW => {
                    // let pnfi = l_param as LPNMLVFINDITEMW;
                    println!("Got find item event!");
                    0

                },
                _ => {
                    user32::DefWindowProcW(hwnd, msg, w_param, l_param)        
                }
            }

        },
        WM_COMMAND => {
            if LOWORD(w_param as u32) as HMENU == IDC_MAIN_EDIT {
                let mut txt = to_wstring("");
                for _ in 0..256 {
                    txt.push(0);
                }

                user32::SendMessageW(EDIT_HWND, 
                    winapi::WM_GETTEXT, 
                    txt.len() as u64,
                    txt.as_ptr() as LPARAM);

                println!("Got Edit Changed txt: {:?}", to_string(&txt));
                return 0;
            }

            user32::DefWindowProcW(hwnd, msg, w_param, l_param)  
        }
        WM_SIZE => {
            // user32::DefWindowProcW(hwnd, msg, w_param, l_param)
            on_size(hwnd, LOWORD(l_param as u32) as i32, HIWORD(l_param as u32) as i32, w_param as u32);
            0
        },
        WM_CLOSE => {
            println!("Got close message");
            user32::DestroyWindow(hwnd) as i64
        }
        WM_DESTROY => {
            println!("Got quit message");
            user32::PostQuitMessage(0);
            0
        },
        4173 => {println!("Got Insert Item"); user32::DefWindowProcW(hwnd, msg, w_param, l_param)}
        4193 => {println!("Got Insert Column"); user32::DefWindowProcW(hwnd, msg, w_param, l_param)}
        _ => user32::DefWindowProcW(hwnd, msg, w_param, l_param)
    }
}

fn on_size(hwnd: HWND, cx: i32, cy: i32, flags: u32) {
    unsafe {
        if cx == 0 {
            println!("OnMove, hwnd: {:?} flags: {:?}", hwnd, flags);
        }

        println!("cx: {:?} cy: {:?}", cx, cy);

        user32::MoveWindow(LIST_HWND, 0, 30, cx, cy-30, 1);
        user32::SendMessageW(LIST_HWND, winapi::LVM_ARRANGE, winapi::LVA_ALIGNTOP, 0);
    }
}


fn test_setup_list(hwnd: HWND) {

    let list_hwnd = create_list_view(hwnd);

    unsafe {
        LIST_HWND = list_hwnd;
    }

    create_column(list_hwnd, "Size", 2);
    create_column(list_hwnd, "Path", 1);
    create_column(list_hwnd, "Name", 0);


    ALL_DATA.write().unwrap().append(&mut get_all_data());
   


    let mut list_write = STRING_CACHE.write().unwrap();

    for e in ALL_DATA.read().unwrap().iter() {
        list_write.insert(e.name.to_string(), to_wstring(&e.name));
        list_write.insert(e.path.to_string(), to_wstring(&e.path));
        let size = lens::pretty_size(e.size());
        list_write.insert(size.to_string(), to_wstring(&size));
    }  
     
    // for (i, e) in ALL_DATA.read().unwrap().iter().enumerate() {
    //     let i = i as i32;
        
    //     insert_item(list_hwnd, &e.name, i);
    //     insert_sub_item(list_hwnd, &e.path, i, 1);
    //     insert_sub_item(list_hwnd, &lens::pretty_size(e.size()), i, 2);


    // }        // user32::MoveWindow(list_hwnd);


    unsafe {
        user32::SendMessageW(list_hwnd, winapi::LVM_SETITEMCOUNT, (ALL_DATA.read().unwrap().len()-1) as u64, 0);

        user32::SendMessageW(LIST_HWND, 
            winapi::LVM_REDRAWITEMS, 
            0,
            10);
        user32::UpdateWindow(LIST_HWND);
    }
}


fn create_column(list_hwnd: HWND, text: &str, sub_item: i32) {
    unsafe {
        let text_column = to_wstring(text);

        let col: winapi::LPLVCOLUMNW = &mut winapi::LV_COLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH, //LVCF_TEXT | LVCF_IDEALWIDTH | LVCF_WIDTH | LVCF_SUBITEM, // | LVCF_ORDER,
            cx: 200,
            pszText: text_column.as_ptr() as LPWSTR,
            cchTextMax: MAX_TEXT_LEN as i32,
            fmt: 0,
            iSubItem: sub_item,
            cxMin: 200,
            cxIdeal: 200,
            cxDefault: 200,
            iImage: 0,
            iOrder: sub_item,
        };

        if user32::SendMessageW(list_hwnd, winapi::LVM_INSERTCOLUMNW, 0, col as LPARAM) != 0 {
            let err = kernel32::GetLastError();
            println!("Failed to send column message, err: {:?}", err);
        } 
    }
}


fn insert_item(list_hwnd: HWND, text: &str, ix: i32) {
    unsafe {
        let text_row = to_wstring(text);

        let item: winapi::LPLVITEMW = &mut winapi::LV_ITEMW {
            mask: winapi::LVIF_TEXT | LVIF_PARAM, // | winapi::LVIF_STATE,
            pszText: LPSTR_TEXTCALLBACKW, //text_row.as_ptr() as LPWSTR, // winapi::LPSTR_TEXTCALLBACKW, // handle_wm_notify as *mut _, //text_row.as_ptr() as *mut _,
            // pszText: text_row.as_ptr() as LPWSTR, // winapi::LPSTR_TEXTCALLBACKW, // handle_wm_notify as *mut _, //text_row.as_ptr() as *mut _,
            iItem: ix,
            iSubItem: 0,
            cchTextMax: 0,
            // cchTextMax: MAX_TEXT_LEN as i32,

            state: 0,
            stateMask: 0,

            
            iImage: 0,
            lParam: ix as i64,

            iIndent: 0,

            cColumns: 0,
            puColumns: std::ptr::null_mut(),
            piColFmt: std::ptr::null_mut(),
            iGroup: 0,
            iGroupId: 0,
        };

        // let ref mut item2: LV_ITEMW = &mut winapi::LV_ITEMW {
        //     mask: winapi::LVIF_TEXT, // | winapi::LVIF_STATE,
        //     pszText: LPSTR_TEXTCALLBACKW, //text_row.as_ptr() as LPWSTR, // winapi::LPSTR_TEXTCALLBACKW, // handle_wm_notify as *mut _, //text_row.as_ptr() as *mut _,
        //     iItem: ix,
        //     iSubItem: sub_item,
        //     cchTextMax: MAX_TEXT_LEN as i32,

        //     state: 0,
        //     stateMask: 0,

            
        //     iImage: 0,
        //     lParam: 0,

        //     iIndent: 0,

        //     cColumns: 0,
        //     puColumns: std::ptr::null_mut(),
        //     piColFmt: std::ptr::null_mut(),
        //     iGroup: 0,
        //     iGroupId: 0,
        // };
        mem::forget(item);
        LIST_CACHE.write().unwrap().push(item as u64);

        if ix == 0 {
            println!("Address for 0: {:?}", item);
        }

        // println!("Insert column: {:?}", winapi::LVM_INSERTCOLUMNW);
        let resp = user32::SendMessageW(list_hwnd, winapi::LVM_INSERTITEMW, 0, item as LPARAM);

        if resp != (ix as i64) {   
            // let err = kernel32::GetLastError();
            println!("Item index does not match!, item: {:?}, return: {:?}", ix, resp);
        }

        // user32::SendMessageW(list_hwnd, winapi::LVM_INSERTITEMW, 0, item as LPARAM);


    }
}

fn insert_sub_item(list_hwnd: HWND, text: &str, ix: i32, sub_item: i32) {
    unsafe {
        let text_row = to_wstring(text);

        let item: winapi::LPLVITEMW = &mut winapi::LV_ITEMW {
            mask: winapi::LVIF_TEXT, // | winapi::LVIF_STATE,
            pszText:  LPSTR_TEXTCALLBACKW,
            // pszText: text_row.as_ptr() as LPWSTR,            
            iItem: ix,
            iSubItem: sub_item,
            cchTextMax: 0,
            // cchTextMax: MAX_TEXT_LEN as i32,

            state: 0,
            stateMask: 0,

            iImage: 0,
            lParam: 0,

            iIndent: 0,

            cColumns: 0,
            puColumns: std::ptr::null_mut(),
            piColFmt: std::ptr::null_mut(),
            iGroup: 0,
            iGroupId: 0,
        };

        mem::forget(item);
        LIST_CACHE.write().unwrap().push(item as u64);

        // println!("Insert column: {:?}", winapi::LVM_INSERTCOLUMNW);
        let resp = user32::SendMessageW(list_hwnd, winapi::LVM_SETITEMW, 0, item as LPARAM);
        if resp != (ix as i64) {
            // let err = kernel32::GetLastError();
            // println!("Sub index does not match!, item: {:?}, return: {:?}", ix, resp);
        }
    }
}


fn create_entry(hwnd_parent: HWND) -> HWND {
       // This should only run once!
    unsafe {
   

        // Setup actual listview
        // let mut rc_client: RECT = RECT {
        //     right: 0, left: 0, top: 0, bottom: 0
        // };
        // user32::GetClientRect(hwnd_parent, &mut rc_client);
        
        let wc = to_wstring("EDIT");

        let style_ex = 0; 
        let style = WS_CHILD | WS_VISIBLE | WS_BORDER;

        let h_instance = kernel32::GetModuleHandleW(std::ptr::null_mut());

        let hwnd = user32::CreateWindowExW(
            style_ex, 
            wc.as_ptr() as *mut _,
            to_wstring("").as_ptr() as *mut _, 
            style, 
            5, 5, 
            250, 
            20, 
            hwnd_parent, 
            IDC_MAIN_EDIT, 
            h_instance,
            std::ptr::null_mut());

        // println!("{:?} x {:?}", rc_client.right - rc_client.left, rc_client.bottom - rc_client.top);        

        println!("Edit hwnd: {:?}", hwnd);
        
        EDIT_HWND = hwnd;

        hwnd
    }
}


fn create_window(title: &str) -> HWND {
    let class_name = to_wstring("my_class");
    let title = to_wstring(title);

    let icon;
    let cursor;
    unsafe {
        icon = user32::LoadIconW(0 as HINSTANCE, winapi::winuser::IDI_APPLICATION);
        cursor = user32::LoadCursorW(0 as HINSTANCE, winapi::winuser::IDI_APPLICATION);
    }

    println!("icon: {:?}", icon);
    println!("cursor: {:?}", cursor);

    let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc), 
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: 0 as HINSTANCE,
        hIcon: icon,
        hCursor: cursor,
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: 0 as LPCWSTR,
        lpszClassName: class_name.as_ptr(),
    };

    let hwnd;

    unsafe {
        // TODO: Check this later
        // let h_instance = GetModuleHandleA(0 as LPCSTR);
        // let hInstance = user32::GetModuleHandleExW(null());

        // We register our class - 
        if user32::RegisterClassW(&wnd) == 0 {
            println!("Failed to register wnd");
            show_message_box("Failed to register wnd");
        }

        let h_wnd_desktop = user32::GetDesktopWindow();

        hwnd = user32::CreateWindowExW(
            0, 
            class_name.as_ptr() as *mut _, 
            title.as_ptr() as *mut _,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE, 
            0, 0, 400, 400,
            h_wnd_desktop ,
            0 as HMENU, 0 as HINSTANCE, std::ptr::null_mut());
        
        println!("wnd: {:?}", h_wnd_desktop);

    }

    println!("hwnd: {:?}", hwnd);
        
    hwnd
}


fn show_message_box(msg: &str) {
    let wmsg = to_wstring(msg);
    unsafe {
        user32::MessageBoxW(null_mut(), wmsg.as_ptr(), wmsg.as_ptr(), winapi::MB_OK);
    }

}


fn create_list_view(hwnd_parent: HWND) -> HWND {
    // This should only run once!
    unsafe {
   

        // Setup actual listview
        let mut rc_client: RECT = RECT {
            right: 0, left: 0, top: 0, bottom: 0
        };
        user32::GetClientRect(hwnd_parent, &mut rc_client);
        
        let wc = to_wstring("SysListView32");
        let h_instance = kernel32::GetModuleHandleW(std::ptr::null_mut());

        let style_ex = 0; //winapi::LVS_EX_AUTOSIZECOLUMNS;
        // let style = WS_CHILD | LVS_REPORT | LVS_EDITLABELS;
        // let style = WS_CHILD | winapi::WS_CLIPCHILDREN | WS_VISIBLE | winapi::LVS_SINGLESEL | LVS_REPORT | winapi::LVS_OWNERDATA | WS_BORDER;
        // let style = WS_CHILD | WS_VISIBLE  | LVS_REPORT | WS_BORDER | winapi::LVS_OWNERDATA ;
        // let style = LVS_REPORT | WS_VISIBLE;

        let style =  WS_VISIBLE | WS_CHILD | WS_TABSTOP |
                  LVS_NOSORTHEADER | LVS_OWNERDATA | LVS_AUTOARRANGE |
                  LVS_SINGLESEL | LVS_REPORT;
        let hwnd = user32::CreateWindowExW(
            style_ex, 
            wc.as_ptr() as *mut _,
            to_wstring("").as_ptr() as *mut _, 
            style, 
            0, 0, 
            rc_client.right - rc_client.left, 
            rc_client.bottom - rc_client.top, 
            hwnd_parent, 
            0 as HMENU, 
            //IDC_MAIN_LISTVIEW, 
            h_instance, 
            std::ptr::null_mut());

        println!("{:?} x {:?}", rc_client.right - rc_client.left, rc_client.bottom - rc_client.top);        

        println!("ListView hwnd: {:?}", hwnd);
        

        hwnd

    }

    // 0 as HWND
}



fn main() {
    // runner::create_bat_file();


    // let msg = "Hello World";
    // let wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    // let ret = unsafe {
    //     user32::MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), winapi::MB_OK)
    // };

    // if ret == 0 {
    //     println!("Failed: {:?}", Error::last_os_error());
    // }
    let title = "my_window";
    let hwnd = create_window(title);

    // test_setup_list(hwnd);

    unsafe {
        user32::ShowWindow(hwnd, winapi::SW_NORMAL);
        user32::UpdateWindow(hwnd);
    }

    let mut msg = winapi::winuser::MSG {
        hwnd : 0 as HWND,
        message : 0 as UINT,
        wParam : 0 as WPARAM,
        lParam : 0 as LPARAM,
        time : 0 as DWORD,
        pt : winapi::windef::POINT { x: 0, y: 0, },
    };

    println!("Starting msg loop");

    loop {
        let pm;
        unsafe {
            pm = user32::GetMessageW(&mut msg, 0 as HWND, 0, 0);
        }

        if pm <= 0 {
            println!("GetMessageW error: {:?}", pm);
            break;
        }

        unsafe {
            user32::TranslateMessage(&mut msg);
            user32::DispatchMessageW(&mut msg);
        }
    }

    println!("Hello, world!");
}
