// Copyright (C) 2022 Anssi Eteläniemi <aetelani@hotmail.com>
use std::ops::{Deref, RangeInclusive};
use slint::{FilterModel, Model, ModelExt, ModelRc, RenderingNotifier, VecModel};
use slint::{Timer, TimerMode};
use rusqlite::{Connection, Result};

slint::slint! {
    import { SpinBox, Button, CheckBox, Slider, LineEdit, ScrollView, ListView,
        HorizontalBox, VerticalBox, TabWidget } from "std-widgets.slint";

    export struct Data := {
        grid-col: int,
        grid-row: int,
        selected: bool,
        uid: string,
    }
    Button := Rectangle {
    property text <=> txt.text;
    callback clicked <=> touch.clicked;
    border-radius: height / 2;
    border-width: 1px;
    border-color: background.darker(25%);
    background: touch.pressed ? #6b8282 : touch.has-hover ? #6c616c :  #456;
    height: txt.preferred-height * 1.33;
    min-width: txt.preferred-width + 20px;
    txt := Text {
        x: (parent.width - width)/2 + (touch.pressed ? 2px : 0);
        y: (parent.height - height)/2 + (touch.pressed ? 1px : 0);
        color: touch.pressed ? #fff : #eee;
    }
    touch := TouchArea { }
    }

    MainWindow := Window {
        //preferred-width: 400px;
        //preferred-height: 600px;
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
        info-show-range(begin, end) => { info.text = "Range of: " + abs(end - begin + 1); }
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
                sv := ScrollView {
                    preferred-width: 400px;
                    preferred-height: 600px;
                    viewport-width: 400px;
                    viewport-height: 600px;
                for it[ind] in model:
                    rect := Rectangle {
                        property <bool> selected: false;
                        x: it.grid-col * 100px;
                        y: { sv.viewport-height = it.grid-row * 20px; it.grid-row * 20px }
                        height: txt.preferred-height * 1.1;
                        width: txt.preferred-width * 1.1;
                        border-width: 1px;
                        background: white;
                        txt := Text {
                            text: model[ind].uid;
                            visible: true;
                            color: it.selected ? red : black;
                        }
                        touch := TouchArea { clicked => {
                            if (it.selected) {
                                it.selected = false;
                                range-select-started-from = -1;
                                info-hide();
                            } else {
                                if (range-select-started-from == -1) {
                                    range-select-started-from = ind;
                                    it.selected = true;
                                    info-show(ind, rect.x, rect.y);
                                } else if (range-select-started-from != -1) {
                                    range-select(range-select-started-from, ind, true);
                                    info-show-range(range-select-started-from, ind);
                                    range-select-started-from = -1;
                                }
                            }
                        }
                    }
                    states [
                        mouse-over when touch.has-hover: {
                            rect.background: { lightgrey };
                        }
                    ]
                    }
                }
            }
        }
        Tab {
            title: "Config";
            Rectangle { background: lightgray;

                }
        }
        }
    }
}
thread_local! {
    static CONN: Connection = Connection::open_in_memory().unwrap();
}

#[cfg_attr(target_arch = "wasm32-wasi",
wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn main() {
    let handle: MainWindow = MainWindow::new();
    let handle_weak = handle.as_weak();
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    let timer = Timer::default();
    let mut row: i32 = 0;
    let mut col: i32 = 0;
    let mut count: usize = 0;
    let max_growth = 5usize;
    create_tables();
    handle_clone.unwrap().on_range_select(move |b: i32, e: i32, mode: bool|{
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
    });
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
        let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
        model.push(Data{ selected: false, grid_col:col as i32, grid_row: row, uid: format!("{0:08x}", count).into()});
        if count % max_growth == max_growth - 1 { row += 1; col = 0; }
        else { col += 1; }
        count += 1;
        ticket_encoded(count);
    });
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    handle_clone.unwrap().on_running(move |v| { if v { timer.restart(); } else { timer.stop() } });
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    handle_clone.unwrap().on_selection(move |v| {
        let mut return_count:i32 = 0;
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
            model.iter().enumerate().filter(|v| v.1.selected).for_each(|(i,_)| idx.push(i));
            idx.reverse();
            for i in idx {
                model.remove(i);
                return_count += 1;
            }
            // Gaps in the list. Redo or repaint
        } else if v == 2 { // Count selected
            let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
            let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
            return_count = model.iter().enumerate().filter(|v| v.1.selected).count() as i32;
        }
        return_count

    });

    handle.run();
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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ticket (\
            uid INTEGER PRIMARY KEY,\
            data TEXT,
            ts TIMESTAMP DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) NOT NULL)",
            (), // empty list of parameters.
        ).unwrap();
        // Much faster when sorting with ts
        conn.execute(
            "CREATE INDEX IF NOT EXISTS ticket_ts_idx ON ticket (ts)",
            (), // empty list of parameters.
        ).unwrap();
    });
}