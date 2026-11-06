use super::*;
use crate::el::{GrowthSpec, ScrollSpec};

const ROW: f32 = 36.0;
const VIEW: f32 = 900.0;

/// `count` positional rows 36 points tall, filling a 900-point window.
fn rows_view(count: &usize, _: &mut Cx<usize>) -> El {
    col().w_full().h_full().child(
        col().key("scroll").w_full().grow(1.0).scroll(ScrollSpec { stick_bottom: false })
            .on_scroll_offset(|at: crate::ScrollPos| TOLD.set(Some(at)))
            .child(col().key("rows").w_full().growth(GrowthSpec::default())
                .virtual_rows_positional(*count, ROW, |_: &usize, _| el().w_full().h(ROW))),
    )
}

thread_local! {
    static TOLD: std::cell::Cell<Option<crate::ScrollPos>> = const { std::cell::Cell::new(None) };
}

fn list(count: usize) -> Runtime<usize> {
    let mut rt = Runtime::new(count, rows_view, (400.0, VIEW), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

fn settle(rt: &mut Runtime<usize>) {
    for _ in 0..240 {
        if !rt.frame(1.0 / 120.0) { return; }
    }
}

/// The rows drawn in the viewport, by index, with their drawn rects.
fn shown(rt: &Runtime<usize>) -> Vec<(usize, [f32; 4])> {
    let mut rows: Vec<_> = rt.elements().into_iter()
        .filter(|e| !e.exiting && e.visible[3] > 0.0)
        .filter_map(|e| Some((e.path.strip_prefix("#0/scroll/rows/")?.parse::<usize>().ok()?, e.rect)))
        .collect();
    rows.sort_by_key(|r| r.0);
    rows
}

/// The rows in view are consecutive, 36 points tall and 36 apart, and tile the viewport.
fn assert_rows_tile(rt: &Runtime<usize>, at: &str) -> Vec<(usize, [f32; 4])> {
    let rows = shown(rt);
    assert!(rows.len() >= (VIEW / ROW) as usize, "{at}: {} rows in view", rows.len());
    for w in rows.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        assert_eq!(b, a + 1, "{at}: rows {a} and {b} in view are not consecutive");
        assert!((rb[1] - ra[1] - ROW).abs() < 0.01, "{at}: row {b} starts {} below row {a}, not {ROW}", rb[1] - ra[1]);
        assert!((ra[3] - ROW).abs() < 0.01, "{at}: row {a} is {} tall", ra[3]);
    }
    let (first, last) = (rows[0].1, rows[rows.len() - 1].1);
    assert!(first[1] <= 0.01 && last[1] + last[3] >= VIEW - 0.01, "{at}: rows span {}..{}", first[1], last[1] + last[3]);
    rows
}

fn reveal(rt: &mut Runtime<usize>, key: usize) {
    rt.update(|_, tx| tx.scroll_into_view(key.to_string()));
    settle(rt);
}

fn huge_list_draws_rows_on_their_pitch_everywhere(count: usize) {
    let mut rt = list(count);
    assert_rows_tile(&rt, "head");

    reveal(&mut rt, 2_000_000);
    let rows = assert_rows_tile(&rt, "row 2,000,000");
    assert!(rows.iter().any(|r| r.0 == 2_000_000), "row 2,000,000 is in view: {:?}", rows.first());
    for _ in 0..20 {
        rt.wheel(200.0, 450.0, -97.0);
        rt.frame(1.0 / 120.0);
    }
    settle(&mut rt);
    assert_rows_tile(&rt, "wheeled past row 2,000,000");

    rt.update(|_, tx| tx.scroll_to_bottom_instant("scroll"));
    settle(&mut rt);
    let rows = assert_rows_tile(&rt, "the end");
    let (last, r) = rows[rows.len() - 1];
    assert_eq!(last, count - 1, "the last row is in view");
    assert!((r[1] + r[3] - VIEW).abs() < 0.01, "the last row's bottom meets the viewport's: {}", r[1] + r[3]);
    let told = TOLD.get().expect("on_scroll_offset heard");
    let end = count as f64 * f64::from(ROW) - f64::from(VIEW);
    let ulp = (end as f32 - f32::from_bits((end as f32).to_bits() - 1)) as f64;
    assert!((f64::from(told.offset[1]) - end).abs() <= ulp && (f64::from(told.max[1]) - end).abs() <= ulp,
        "on_scroll_offset tells the offset and extent from the top: {told:?} vs {end}");

    rt.key(crate::KeyEvent { key: crate::Key::Home, mods: crate::Mods::default() });
    rt.frame(1.0 / 120.0);
    let rows = assert_rows_tile(&rt, "Home");
    assert!(rows[0].0 == 0 && rows[0].1[1] == 0.0, "Home reaches the first row: {:?}", rows[0]);
    rt.key(crate::KeyEvent { key: crate::Key::End, mods: crate::Mods::default() });
    rt.frame(1.0 / 120.0);
    let rows = assert_rows_tile(&rt, "End");
    let (last, r) = rows[rows.len() - 1];
    assert!(last == count - 1 && (r[1] + r[3] - VIEW).abs() < 0.01, "End reaches the last row: {:?}", rows.last());

    for _ in 0..12 {
        rt.wheel(200.0, 450.0, 4_500_000.0);
        rt.frame(1.0 / 120.0);
    }
    settle(&mut rt);
    let rows = assert_rows_tile(&rt, "wheeled up 54M points");
    let told = f64::from(TOLD.get().unwrap().offset[1]);
    assert!((told - (end - 54_000_000.0)).abs() <= ulp, "a turn longer than the window moves by all of it: {told} vs {}", end - 54_000_000.0);
    assert_eq!(rows[0].0, ((end - 54_000_000.0) / f64::from(ROW)) as usize);

    reveal(&mut rt, 1_000);
    assert_rows_tile(&rt, "back near the top");
    reveal(&mut rt, count - 1);
    let rows = assert_rows_tile(&rt, "the last row revealed");
    let (last, r) = rows[rows.len() - 1];
    assert_eq!(last, count - 1);
    assert!((r[1] + r[3] - VIEW).abs() < 0.01, "a revealed last row sits at the viewport's bottom: {}", r[1] + r[3]);
}

#[test]
fn a_4_4m_row_list_draws_rows_on_their_pitch_everywhere() {
    huge_list_draws_rows_on_their_pitch_everywhere(4_400_000);
}

#[test]
fn a_100m_row_list_draws_rows_on_their_pitch_everywhere() {
    huge_list_draws_rows_on_their_pitch_everywhere(100_000_000);
}

/// `count` positional rows below a 100-point header, in the same scroll content.
fn framed_view(count: &usize, _: &mut Cx<usize>) -> El {
    col().w_full().h_full().child(
        col().key("scroll").w_full().grow(1.0).scroll(ScrollSpec { stick_bottom: false })
            .on_scroll_offset(|at: crate::ScrollPos| TOLD.set(Some(at)))
            .children([
                el().key("head").w_full().h(100.0),
                col().key("rows").w_full().growth(GrowthSpec::default())
                    .virtual_rows_positional(*count, ROW, |_: &usize, _| el().w_full().h(ROW)),
            ]),
    )
}

fn rect(rt: &Runtime<usize>, key: &str) -> [f32; 4] {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).map(|e| e.rect).expect(key)
}

/// Every element in view draws where the offset `on_scroll_offset` reports puts it.
fn assert_drawn_at_told(rt: &Runtime<usize>, at: &str) {
    let told = f64::from(TOLD.get().expect("on_scroll_offset heard").offset[1]);
    let mut checked = 0;
    for e in rt.elements().into_iter().filter(|e| !e.exiting && e.visible[3] > 0.0) {
        let top = match e.path.strip_prefix("#0/scroll/") {
            Some("head") => 0.0,
            Some(row) => match row.strip_prefix("rows/").and_then(|i| i.parse::<f64>().ok()) {
                Some(i) => 100.0 + i * f64::from(ROW),
                None => continue,
            },
            None => continue,
        };
        assert!((f64::from(e.rect[1]) - (top - told)).abs() < 0.05, "{at}: {} draws at {}, not {}", e.path, e.rect[1], top - told);
        checked += 1;
    }
    assert!(checked >= 20, "{at}: {checked} elements in view");
}

#[test]
fn rows_and_header_draw_where_the_offset_says_as_the_layout_origin_moves() {
    // At the end, the window starts 80 points below an origin step: scrolling up moves the origin
    // with the last rows in view.
    let count = 7359;
    let mut rt = Runtime::new(count, framed_view, (400.0, VIEW), Env::new(true, 2.0));
    rt.frame(0.0);
    assert_drawn_at_told(&rt, "top");
    let origin = |rt: &Runtime<usize>| rt.virtuals["#0/scroll/rows"].origin;
    rt.update(|_, tx| tx.scroll_to_bottom_instant("scroll"));
    settle(&mut rt);
    assert_drawn_at_told(&rt, "bottom");
    let last = rect(&rt, &(count - 1).to_string());
    assert!((last[1] + last[3] - VIEW).abs() < 0.01, "the last row ends the content: {last:?}");
    let (at_end, mut moved) = (origin(&rt), 0);
    assert!(at_end > 0.0, "the end lays out from an origin below the top");
    for step in 0..40 {
        let was = origin(&rt);
        rt.wheel(200.0, 450.0, 50.0);
        rt.frame(1.0 / 120.0);
        assert_drawn_at_told(&rt, &format!("up {step}"));
        if origin(&rt) != was { moved += 1; }
    }
    assert!(moved > 0, "the origin moved while the last rows showed");
    for step in 0..100 {
        rt.wheel(200.0, 450.0, 3000.0);
        rt.frame(1.0 / 120.0);
        assert_drawn_at_told(&rt, &format!("up fast {step}"));
    }
    settle(&mut rt);
    assert_eq!(origin(&rt), 0.0, "back at the top");
    assert_drawn_at_told(&rt, "top again");
    assert_eq!(rect(&rt, "head")[1], 0.0);
}

#[test]
fn wheeling_down_a_long_list_moves_every_row_by_the_offset() {
    let count = 20_000;
    let mut rt = Runtime::new(count, framed_view, (400.0, VIEW), Env::new(true, 2.0));
    rt.frame(0.0);
    let mut origins = std::collections::BTreeSet::new();
    let mut last = 0.0;
    for step in 0..300 {
        rt.wheel(200.0, 450.0, -2000.0);
        rt.frame(1.0 / 120.0);
        assert_drawn_at_told(&rt, &format!("down {step}"));
        let told = TOLD.get().unwrap().offset[1];
        assert!(told > last, "down {step}: the offset went from {last} to {told}");
        last = told;
        origins.insert(rt.virtuals["#0/scroll/rows"].origin.to_bits());
    }
    assert!(origins.len() > 5, "the walk crossed origin steps: {} origins", origins.len());
}
