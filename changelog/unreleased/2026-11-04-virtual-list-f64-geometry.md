- **Virtual lists draw rows on their true pitch however far down they are.** A list's row
  offsets are f64, and once its window is more than 131,072 points down, layout sees the rows
  from an origin a little above the window, so no laid-out or drawn position is ever that large.
  Before, the f32 geometry rounded positions far down (to 8 points at 72M points, 16 at 158M):
  at row 2,000,000 of 36-point rows the rows drew 32 points apart and each covered the last's
  bottom edge, and the last row of a 4.4M-row list stopped about 350 points short of the
  viewport's bottom. `on_scroll_offset`, accessibility and `Tx::scroll_to` offsets still count
  from the content's top.
