use slint::{Model, ModelRc, VecModel};
use slint::{Timer, TimerMode};
use rusqlite::{Connection, Result};

slint::slint! {
    import { SpinBox, Button, CheckBox, Slider, LineEdit, ScrollView, ListView,
        HorizontalBox, VerticalBox, StandardButton, GridBox } from "std-widgets.slint";

    export struct TicketItem := {
        uid: string,
    }

    MainWindow := Window {
        preferred-width: 400px;
        preferred-height: 600px;
        property <[TicketItem]> ticket-model: [];
        callback update-model();
        update-model() => { it-row = 0; }

        property <int> it-row: 0;
        property<int> it-col: -1;

        for it[ind] in ticket-model:
            Text {
                x: { it-col += 1; it-col * 30px }
                y: {
                    debug(mod(ind,3));
                    if (mod(ind, 3) == 0) {
                        it-row += 1;
                        it-col = -1;
                    }
                    it-row * 20px
                }
                text: { if (visible) { it.uid + "v" } else { it.uid + "h" } };
                visible: false;
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
    create_tables();
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
        let model_handle = handle_clone.unwrap().get_ticket_model();
        let model = model_handle.as_any().downcast_ref::<VecModel<TicketItem>>().unwrap();
        model.push(TicketItem{ uid: format!("{0}", count).into()});
        ticket_encoded(count);
        count += 1;
        handle_clone.unwrap().invoke_update_model();
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