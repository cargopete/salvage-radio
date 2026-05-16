//! Startup warm-up animation — ~600ms total.
//!
//! Sequence (once per session, sets the tone):
//!   1.  Empty frame                                                (~0ms)
//!   2.  ▓ blocks fill the status bar left-to-right               (~300ms)
//!   3.  Dial illuminates: freq markings appear, callsigns fade
//!       tarnish → brass                                           (~150ms)
//!   4.  Station list populates line by line                       (~20ms/line)
//!   5.  Now-playing panel loads with most recent broadcast        (instant)
//!
//! This is the only other piece of motion besides the new-broadcast flicker.
//! It happens once. It should feel like a machine coming to life, not a
//! loading spinner.

// TODO M4: implement as an async function driven by tokio::time::sleep.
// Each step is a full ratatui terminal.draw() call.
// Example skeleton:
//
// pub async fn play(terminal: &mut Terminal<impl Backend>, stations: &[DialStation]) -> Result<()> {
//     // step 2: fill status bar
//     for col in 0..terminal_width {
//         terminal.draw(|f| render_warmup_step2(f, col))?;
//         sleep(Duration::from_millis(300 / terminal_width as u64)).await;
//     }
//     // step 3: dial
//     for brightness in 0..=4 {
//         terminal.draw(|f| render_warmup_step3(f, brightness, stations))?;
//         sleep(Duration::from_millis(37)).await;
//     }
//     // step 4: station list
//     for i in 0..stations.len() {
//         terminal.draw(|f| render_warmup_step4(f, i, stations))?;
//         sleep(Duration::from_millis(20)).await;
//     }
//     Ok(())
// }
