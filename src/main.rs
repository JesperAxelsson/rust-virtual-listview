#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]

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

use std::ffi::OsStr;
use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::ptr;
use std::mem;
use std::collections::HashMap;

use widestring::{WideString, WideCString};

use winapi::*;



struct Row {
    item: String, 
    sub_item: String,
}

fn get_all_data() -> Vec<Row> {

    let mut vec = Vec::with_capacity(100);

    for i in 0..100 {
        vec.push(Row {
            item: format!("Item: {:?}", i),
            sub_item: format!("SubItem {:?}", i),
        });
    }

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
    static ref ALL_DATA: RwLock<Vec<Row>> = RwLock::new(Vec::new());
    static ref STRING_CACHE: RwLock<HashMap<String, Vec<u16>>> = RwLock::new(HashMap::new());
}

static mut LIST_HWND: HWND = 0 as HWND;

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
            
            test_setup_list(hwnd);

            println!("Running WM_CREATE");
            0
        },

        WM_NOTIFY => {
         
            match (*(l_param as LPNMHDR) as NMHDR).code {
                LVN_GETDISPINFOW => {
           
                    let ix: usize = (*(l_param as *const NMLVDISPINFOW)).item.iItem as usize;
                    let mask = (*(l_param as *const NMLVDISPINFOW)).item.mask;

                    if (*(l_param as *const NMLVDISPINFOW)).item.iItem < 0 {
                        println!("Request Item with negative index:{:?}", (*(l_param as *const NMLVDISPINFOW)).item.iItem);
                        return 0;
                    }
                   
                    if (mask & LVIF_TEXT) == LVIF_TEXT {
                        let len = ALL_DATA.read().unwrap().len();
                        if ix >= len -1 {
                            println!("ix bigger then index! ix: {:?} len: {:?}", ix, len );
                            return 0;

                        }
                        let ref item = ALL_DATA.read().unwrap()[ix];

                        let f = |key| {
                            let ref vec = STRING_CACHE.read().unwrap()[(key)];
                            let ptr = STRING_CACHE.read().unwrap()[(key)].as_ptr();
                            (ptr, vec.len())
                        };

   
                        let (ptr, _) = match (*(l_param as *const NMLVDISPINFOW)).item.iSubItem {
                            0 => f(&item.item),
                            1 => f(&item.sub_item),
                            n => {
                                println!("Found subitem: {:?}", n);
                                return 0;
                            }
                        };

                        (*(l_param as *mut NMLVDISPINFOW)).item.pszText = ptr as LPWSTR; 
                                        
                    }

                    0
                    
                },
                LVN_ODCACHEHINT=> {
                    user32::DefWindowProcW(hwnd, msg, w_param, l_param)
                },
                LVN_ODFINDITEMW => {
                    // let pnfi = l_param as LPNMLVFINDITEMW;
                    println!("Got find item event!");
                    user32::DefWindowProcW(hwnd, msg, w_param, l_param)

                },
                _ => {
                    user32::DefWindowProcW(hwnd, msg, w_param, l_param)
                }
            }

        },
        WM_SIZE => {
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

    create_column(list_hwnd, "SubItem", 1);
    create_column(list_hwnd, "Item", 0);


    ALL_DATA.write().unwrap().append(&mut get_all_data());
   

    // Create the cache
    let mut list_write = STRING_CACHE.write().unwrap();

    for e in ALL_DATA.read().unwrap().iter() {
        list_write.insert(e.item.to_string(), to_wstring(&e.item));
        list_write.insert(e.sub_item.to_string(), to_wstring(&e.sub_item));
    }  

    unsafe {
        user32::SendMessageW(list_hwnd, winapi::LVM_SETITEMCOUNT, (ALL_DATA.read().unwrap().len()-1) as u64, 0);
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
        // We register our class - 
        if user32::RegisterClassW(&wnd) == 0 {
            println!("Failed to register wnd");
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


fn create_list_view(hwnd_parent: HWND) -> HWND {
    // This should only run once!
    unsafe {
   
        // Setup actual listview
        
        let wc = to_wstring("SysListView32");
        let h_instance = kernel32::GetModuleHandleW(std::ptr::null_mut());

        let style =  WS_VISIBLE | WS_CHILD | WS_TABSTOP |
                  LVS_NOSORTHEADER | LVS_OWNERDATA | LVS_AUTOARRANGE |
                  LVS_SINGLESEL | LVS_REPORT;
        let hwnd = user32::CreateWindowExW(
            0, 
            wc.as_ptr() as *mut _,
            to_wstring("").as_ptr() as *mut _, 
            style, 
            0, 0, 0, 0,
            hwnd_parent, 
            IDC_MAIN_LISTVIEW , 
            h_instance, 
            std::ptr::null_mut());

        hwnd

    }
}



fn main() {
    let title = "my_window";
    let hwnd = create_window(title);

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
