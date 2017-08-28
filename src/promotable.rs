use std::f64::consts::PI;

use time::SteadyTime;

use gtk::prelude::*;
use cairo::Context;
use rsvg::HandleExt;

use shakmaty::{Square, Color, Role};

use util::{ease, square_to_pos};
use pieces::Pieces;
use board_state::BoardState;
use ground::{WidgetContext, EventContext, GroundMsg};

pub struct Promotable {
    promoting: Option<Promoting>,
}

struct Promoting {
    orig: Square,
    dest: Square,
    hover: Option<Square>,
    time: SteadyTime,
}

impl Promotable {
    pub fn new() -> Promotable {
        Promotable {
            promoting: None,
        }
    }

    pub fn start_promoting(&mut self, orig: Square, dest: Square) {
        self.promoting = Some(Promoting {
            orig,
            dest,
            hover: Some(dest),
            time: SteadyTime::now(),
        });
    }

    pub fn is_promoting(&self, orig: Square) -> bool {
        self.promoting.as_ref().map_or(false, |p| p.orig == orig)
    }

    pub fn is_animating(&self) -> bool {
        self.promoting.as_ref().map_or(false, |p| {
            p.hover.is_some() && p.elapsed() < 1.0
        })
    }

    pub(crate) fn queue_animation(&self, ctx: &WidgetContext) {
        if let Some(Promoting { hover: Some(square), .. }) = self.promoting {
            ctx.queue_draw_square(square);
        }
    }

    pub(crate) fn mouse_move(&mut self, ctx: &EventContext) {
        self.queue_animation(ctx.widget());

        if let Some(ref mut promoting) = self.promoting {
            if promoting.hover != ctx.square() {
                promoting.hover = ctx.square();
                promoting.time = SteadyTime::now();

            }
        }

        self.queue_animation(ctx.widget());
    }

    pub(crate) fn mouse_down(&mut self, pieces: &mut Pieces, ctx: &EventContext) -> Inhibit {
        if let Some(promoting) = self.promoting.take() {
            ctx.widget().queue_draw();

            // animate the figurine when cancelling
            if let Some(figurine) = pieces.figurine_at_mut(promoting.orig) {
                figurine.pos = square_to_pos(promoting.dest);
                figurine.time = SteadyTime::now();
            }

            if let Some(square) = ctx.square() {
                let side = promoting.orientation();

                if square.file() == promoting.dest.file() {
                    let role = match square.rank() {
                        r if r == side.fold(7, 0) => Some(Role::Queen),
                        r if r == side.fold(6, 1) => Some(Role::Rook),
                        r if r == side.fold(5, 2) => Some(Role::Bishop),
                        r if r == side.fold(4, 3) => Some(Role::Knight),
                        r if r == side.fold(3, 4) => Some(Role::King),
                        r if r == side.fold(2, 5) => Some(Role::Pawn),
                        _ => None,
                    };

                    if role.is_some() {
                        ctx.stream().emit(GroundMsg::UserMove(promoting.orig, promoting.dest, role));
                        return Inhibit(true);
                    }
                }
            }
        }

        Inhibit(false)
    }

    pub(crate) fn draw(&self, cr: &Context, state: &BoardState) {
        self.promoting.as_ref().map(|p| p.draw(cr, state));
    }
}

impl Promoting {
    fn elapsed(&self) -> f64 {
        (SteadyTime::now() - self.time).num_milliseconds() as f64 / 1000.0
    }

    fn orientation(&self) -> Color {
        Color::from_bool(self.dest.rank() > 4)
    }

    fn draw(&self, cr: &Context, state: &BoardState) {
        // make the board darker
        cr.rectangle(0.0, 0.0, 8.0, 8.0);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.fill();

        for (offset, role) in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::King, Role::Pawn].iter().enumerate() {
            if !state.legal_move(self.orig, self.dest, Some(*role)) {
                continue;
            }

            let rank = self.orientation().fold(7 - offset as i8, offset as i8);
            let light = self.dest.file() + rank & 1 == 1;

            cr.save();
            cr.rectangle(self.dest.file() as f64, 7.0 - rank as f64, 1.0, 1.0);
            cr.clip_preserve();

            // draw background
            if light {
                cr.set_source_rgb(0.25, 0.25, 0.25);
            } else {
                cr.set_source_rgb(0.18, 0.18, 0.18);
            }
            cr.fill();

            // draw piece
            let radius = match self.hover {
                Some(hover) if hover.file() == self.dest.file() && hover.rank() == rank => {
                    let elapsed = self.elapsed();

                    cr.set_source_rgb(ease(0.69, 1.0, elapsed),
                                      ease(0.69, 0.65, elapsed),
                                      ease(0.69, 0.0, elapsed));

                    ease(0.5, 0.5f64.hypot(0.5), elapsed)
                },
                _ => {
                    cr.set_source_rgb(0.69, 0.69, 0.69);
                    0.5
                },
            };

            cr.arc(0.5 + self.dest.file() as f64, 7.5 - rank as f64, radius, 0.0, 2.0 * PI);
            cr.fill();

            cr.translate(0.5 + self.dest.file() as f64, 7.5 - rank as f64);
            cr.scale(2f64.sqrt() * radius, 2f64.sqrt() * radius);
            cr.translate(-0.5, -0.5);
            cr.scale(state.piece_set.scale(), state.piece_set.scale());
            state.piece_set.by_piece(&role.of(self.orientation())).render_cairo(cr);

            cr.restore();
        }
    }
}
