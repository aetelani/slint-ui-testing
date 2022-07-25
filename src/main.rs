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
        preferred-width: 400px;
        preferred-height: 600px;
        property <[Data]> model: [];
        for it[ind] in model:
            rect := Rectangle {
                x: it.grid-col * txt.preferred-width * 1.4; // FIXed: Just use the pre-count values
                y: it.grid-row * 20px;
                height: txt.preferred-height * 1.1;
                width: txt.preferred-width * 1.1;
                border-width: 1px;
                txt := Text {
                    text: model[ind].uid ;
                    visible: true;
                    color: touch.pressed ? red : black;
                }
                touch := TouchArea { }
                states [
                    mouse-over when touch.has-hover: {
                        rect.background: lightgrey;
                    }
                    mouse-not-over when !touch.has-hover: {
                        rect.background: white;
                    }
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
    let handle_clone: slint::Weak<MainWindow> = handle_weak.clone();
    let timer = Timer::default();
    let mut count: usize = 0;
    let mut row: i32 = 0;
    let mut col: i32 = 0;
    let max_growth = 5usize;
    create_tables();
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        let model_handle: ModelRc<Data> = handle_clone.unwrap().get_model();
        let model: &VecModel<Data> = model_handle.as_any().downcast_ref::<VecModel<Data>>().unwrap();
        model.push(Data{ grid_col:col as i32, grid_row: row, uid: format!("{0:08x}", count).into()});
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