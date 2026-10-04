//! Space-side consequences of a boarding action (shared by the scene and the playtest bot).

use super::event::Event;
use super::world::{EntityId, World};

/// Fold a finished boarding action into the space world: loot, casualties, torpedo/missile
/// resupply on success, and removal of the captured or sabotaged ship. Returns the crew lost.
pub fn apply(w: &mut World, target: EntityId, won: bool, credits: i32, damage_taken: f32) -> u32 {
    let casualties = (damage_taken / 25.0).round() as u32;
    if let Some(o) = w
        .player
        .and_then(|p| w.get_mut(p))
        .and_then(|e| e.ops.as_deref_mut())
    {
        o.cargo.credits += credits;
        o.sys.crew = o.sys.crew.saturating_sub(casualties);
        if won {
            o.torpedoes = (o.torpedoes + 2).min(o.max_torpedoes());
            o.missiles = (o.missiles + 3).min(o.max_missiles());
        }
    }
    if won && let Some(t) = w.get(target).map(|e| e.pos) {
        w.push_event(Event::Explosion { pos: t, size: 60.0 });
        w.despawn(target);
    }
    casualties
}
