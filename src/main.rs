// Copyright (C) 2022 Anssi Eteläniemi <aetelani@hotmail.com>
use slint::{Model, ModelRc, VecModel};
use slint::{Timer, TimerMode};
use rusqlite::{Connection, Result};

slint::slint! {
    import { SpinBox, Button, CheckBox, Slider, LineEdit, ScrollView, ListView,
        HorizontalBox, VerticalBox, StandardButton, GridBox } from "std-widgets.slint";

    export struct Data := {
        grid-col: int,
        grid-row: int,
        uid: string,
    }

    MainWindow := Window {
        preferred-width: 400px;
        preferred-height: 600px;
        property <int> grid-col: 0;
        property <int> grid-row: 0;
        property <int> grid-growth-max: 5;
        property <int> grid-row-increase: 0;
        property <int> grid-col-increase: 0;
        property <[Data]> model: [];
        callback inc-col(int, int); inc-col(v, a) => { grid-col = v + a; }
        callback inc-row(int, int); inc-row(v, a) => { grid-row = v + a; }
        callback inc-col-sem(int, int); inc-col-sem(v, a) => { grid-col-increase = v + a; }
        callback inc-row-sem(int, int); inc-row-sem(v, a) => { grid-row-increase = v + a; }
        callback update-grid(int);
        update-grid(ind) => {
            if (ind < grid-growth-max) {
                grid-col = ind;
                grid-row = 0;
                return;
            } else if (ind == grid-growth-max) {
                grid-col = 0;
                grid-row = 1;
                grid-row-increase = 0;
                grid-col-increase = 3;
                return;
            }
            if (ind == model.length - 1) {
                grid-row-increase = 0;
                grid-col-increase = 0;
                return; }
            if (grid-row-increase == 3) {
                //grid-row += 1;
                inc-row(grid-row, 1);
                grid-col = 0;
                grid-row-increase = 0;
                grid-col-increase = 0;
            }
            if (grid-col-increase == 3) {
                //grid-col += 1;
                inc-col(grid-col, 1);
                grid-col-increase = 0;
            }
            // Conditions
            if (mod(ind, grid-growth-max) == grid-growth-max - 1) {
                //grid-row-increase += 1;
                inc-row-sem(grid-row-increase, 1);
            } else if (ind == model.length - 1) {
            } else {
                inc-col-sem(grid-col-increase, 1);
                //grid-col-increase += 1;
            }
        }
        for it[ind] in model:
            Text {
                x: { update-grid(ind); it.grid-col * 20px } // FIXed: Just use the pre-count values
                y: { update-grid(ind); it.grid-row * 20px }
                //text: { "("+grid-col+","+grid-row+")" };
                text: { model[ind].uid };
                visible: false;
                states [
                    //start-grid when ind == 0: { y: 0; x: 0; }
                    //last-col when (mod(ind, grid-growth-max) == grid-growth-max - 1): { y: y + 20px; x: 0px; }
                    //start-col when (mod(ind, grid-growth-max) == 0): { x: 0px; y: y + 20px; }
                    //next-col when (mod(ind, grid-growth-max) != 0): { x: x + 20px; }
                ]
            }
    }
}
thread_local! {
    static CONN: Connection = Connection::open_in_memory().unwrap();
}

pub fn main() {
    let handle = MainWindow::new();
    let handle_weak = handle.as_weak();
    let handle_clone = handle_weak.clone();
    let timer = Timer::default();
    let mut count: usize = 0;
    let mut row: i32 = 0;
    let mut col: i32 = 0;
    let max_growth = 5usize;
    create_tables();
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
        let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
        model.push(Data{ grid_col:col as i32, grid_row: row, uid: format!("{0}", count).into()});
        if count % max_growth == max_growth - 1 { row += 1; col = 0; }
        else { col += 1; }
        ticket_encoded(count);
        count += 1;
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