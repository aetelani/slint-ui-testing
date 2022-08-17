// Copyright (C) 2022 Anssi Eteläniemi <aetelani@hotmail.com>
use std::ops::{Deref, RangeInclusive};
use std::time::SystemTime;
use slint::{FilterModel, Model, ModelExt, ModelRc, RenderingNotifier, VecModel, Weak};
use slint::{Timer, TimerMode};
use rusqlite::{Connection, Result};

slint::slint! {
    import { SpinBox, Button, CheckBox, Slider, LineEdit, ScrollView, ListView,
        HorizontalBox, VerticalBox, TabWidget } from "std-widgets.slint";

    export global Logic := {
        callback model-remove-rows(int, int);
    }

    export struct Data := {
        selected: bool,
        uid: string,
    }
    MainWindow := Window {
        preferred-width: 710px;
        preferred-height: 400px;
        property <[Data]> model: [];

        property <int> range-select-started-from: -1;
        callback range-select(int, int, bool);
        callback info-show(int, length, length);
        callback info-hide();
        callback info-show-range(int, int);
        callback running(bool);
        callback selection(int) -> int;
        info-show(ind,posx,posy) => { info.text = ind + ":(" + posx/1px + "," + posy/1px  + ")"; }
        info-hide() => { info.text = ""; }
        info-show-range(begin, end) => {
            if (end > begin) {
                info.text = "Range of: " + abs(end - begin + 1);
            } else {
                info.text = "Range of: " + abs(begin - end + 1);
            }
        }

        TabWidget {
            Tab {
            title: "Uids";
                VerticalBox {
              HorizontalBox {
                Button { text: "Start"; clicked() => { running(true); } }
                Button { text: "Stop"; clicked() => { running(false); } }
                Button { text: "Cleanup Selection"; clicked() => { info.text = "Cleaned: " + selection(0); } }
                Button { text: "Delete Selection"; clicked() => { info.text = "Deleted: " + selection(1); } }
                Button { text: "Count Selected"; clicked() => { info.text = "Total Selected: " + selection(2); } }
                info := Text { height: 50px; width: 100px; }
              }
                sv := ListView {
                        for it[ind] in model:
                        rb := Rectangle {
// BUG: Cols does not really work as the table is over-indexed with empty text fields
                                property<[int]> r-model: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
                                hb := HorizontalBox {
                                for r-ind in r-model:
                                rect := Rectangle {
                                    width: txt.width;
                                    height: txt.height;
                                    callback map-ind() -> int;
                                    map-ind() => { (ind * r-model.length) + r-ind }
                                txt := Text {
                                        width: 60px; // Perf issue if not defined
                                        height: 14px;
                                    text:  model[map-ind()].uid;
                                    color: model[map-ind()].selected ? red : black;
                                }
                                touch := TouchArea { clicked => {
                                    if (model[map-ind()].selected) {
                                        model[map-ind()].selected = false;
                                        range-select-started-from = -1;
                                        info-hide();
                                    } else {
                                        if (range-select-started-from == -1) {
                                            range-select-started-from =  map-ind();
                                            model[map-ind()].selected = true;
                                            info-show(map-ind(), rect.x, rect.y);
                                        } else if (range-select-started-from != -1) {
                                            range-select(range-select-started-from, map-ind(), true);
                                            info-show-range(range-select-started-from, map-ind());
                                            range-select-started-from = -1;
                                        }
                                    }
                                }
                                }
                                    states [
                                        mouse-over when touch.has-hover: {
                                            background: { lightgrey };
                                        }
                                    ]
                                }
                            }
                        }
                    }
                }
            }
        Tab {
            title: "Config";
            Rectangle { background: lightgray; }
            }
        }
    }
}
thread_local! {
    static CONN: Connection = Connection::open_in_memory().unwrap();
    static START_TS: SystemTime = SystemTime::now();
}

#[cfg_attr(target_arch = "wasm32-wasi",
wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn main() {
    let handle: MainWindow = MainWindow::new();
    let handle_weak = handle.as_weak();
    let timer = Timer::default();

    // Init database
    create_tables();

    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    handle_clone.unwrap().on_range_select(on_range_select_handler(handle_clone));

    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    handle_clone.unwrap().on_selection(on_selection_handler(handle_clone));

    // Populate data
    let mut count: usize = 0; // Used as UID
    let mut start_ts = SystemTime::now();
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    let mut insert_data = move |print_debug:bool| {
        let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
        let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
        model.insert(0,Data{ selected: false, uid: format!("{0:08x}", count).into()});
        count += 1;
        //ticket_encoded(count);
        let diff= SystemTime::now().duration_since(start_ts).unwrap().as_millis() as usize;
        start_ts = SystemTime::now();
        if print_debug { eprintln!("{count} @ {diff}ms/paint"); }
    };
    /*for _ in 0..2000 {
        let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
        insert_data(false);
    }*/

    // Start timing
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(20), move || {
        insert_data(false);
    });
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    handle_clone.unwrap().on_running(move |v| { if v { timer.restart(); } else { timer.stop() } });

    // Main loop
    handle.run();
}

fn on_selection_handler(handle_clone: Weak<MainWindow>) -> impl FnMut(i32) -> i32 {
    move |v| {
        let mut return_count: i32 = 0;
        if v == 0 { // Unselect
            let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
            let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
            for i in 0..model.row_count() {
                let mut data = model.row_data(i).unwrap();
                if data.selected {
                    data.selected = false;
                    model.set_row_data(i, data);
                    return_count += 1;
                }
            }
        } else if v == 1 { // Remove
            let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
            let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
            let mut idx: Vec<usize> = vec![];
            model.iter().enumerate().filter(|v| v.1.selected).for_each(|(i, _)| idx.push(i));
            idx.reverse();
            for i in idx {
                model.remove(i);
                return_count += 1;
            }
        } else if v == 2 { // Count selected
            let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
            let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
            return_count = model.iter().filter(|v| v.selected).count() as i32;
        }
        return_count
    }
}

fn on_range_select_handler(handle_clone: Weak<MainWindow>) -> impl FnMut(i32, i32, bool) {
    move |b: i32, e: i32, mode: bool| {
        let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
        // range does not work this way so normalizing up selected
        let range: RangeInclusive<usize>;
        if b > e {
            range= e as usize ..=b as usize;
        } else {
            range = b as usize ..=e as usize;
        }
        let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
        for i in range {
            let data_maybe: Option<Data> = model.row_data(i as usize);
            if let Some(mut data) = data_maybe {
                data.selected = mode;
                model.set_row_data(i as usize, data);
            } else {
                dbg!("failed update ind:", i);
            }
        }
    }
}

fn dump_head_ticket() {
    CONN.with(|conn| {
        let mut stmt = conn.prepare("SELECT uid, ts FROM ticket ORDER BY ts DESC LIMIT 1 ").unwrap();
        let it = stmt.query_map([], |row| {
            Ok((row.get::<_, usize>(0).unwrap(), row.get::<_,String>(1usize).unwrap()))
        }).expect("Badly formatted query");
        for i in it { if let Ok((uid, ts)) = i { dbg!(uid, ts);} else {} }
    });
}

fn ticket_encoded(uid: usize) {
    CONN.with(|conn|{
        conn.execute(
            "INSERT INTO ticket (uid) VALUES (?)",
            [uid],
        ).unwrap();
    });
}

fn create_tables() {
    CONN.with(|conn|{
        // rowid is serial starting from 1 gaps are not filled
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ticket (\
            uid INTEGER NOT NULL PRIMARY KEY,\
            sn INTEGER NOT NULL UNIQUE,\
            data TEXT, \
            ts TIMESTAMP DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) NOT NULL)",
            (), // empty list of parameters.
        ).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sn_pool (\
            sn INTEGER NOT NULL PRIMARY KEY)",
            (), // empty list of parameters.
        ).unwrap();
        // Much faster when sorting with ts
        conn.execute(
            "CREATE INDEX IF NOT EXISTS ticket_ts_idx ON ticket (ts)",
            (), // empty list of parameters.
        ).unwrap();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS ticket_ts_idx ON ticket (sn)",
            (), // empty list of parameters.
        ).unwrap();
    });
}