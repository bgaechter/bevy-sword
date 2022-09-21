
use super::MapArchitect;
use crate::prelude::*;
pub struct RandomWalkArchitect {}

const MAX_WALKING_DISTANCE: usize = 400;
const NUM_TILES: usize = (SCREEN_WIDTH * SCREEN_HEIGHT) as usize;
const DESIRED_FLOOR : usize = NUM_TILES / 3;

impl MapArchitect for RandomWalkArchitect {
    fn new(&mut self, rng: &mut RandomNumberGenerator) -> MapBuilder {
        let mut mb = MapBuilder {
            map: Map::new(),
            rooms: Vec::new(),
            monster_spawns: Vec::new(),
            player_start: Point::zero(),
            amulet_start: Point::zero(),
            // theme: super::themes::DungeonTheme::new()
        };
        mb.fill(TileType::Wall);
        let center = Point::new(SCREEN_WIDTH /2, SCREEN_HEIGHT/2);
        self.walk(&center, rng, &mut mb.map);
        while mb.map.tiles.iter()
            .filter(|t| **t == TileType::Floor).count() < DESIRED_FLOOR
        {
            self.walk(
                &Point::new(
                    rng.range(0, SCREEN_WIDTH),
                    rng.range(0, SCREEN_HEIGHT)
                ),
                rng,
                &mut mb.map
            );
            let dijkstra_map = DijkstraMap::new(// (3)
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &vec![mb.map.point2d_to_index(center)],
                &mb.map,
                1024.0
            );
            dijkstra_map.map// (4)
                .iter()
                .enumerate()
                .filter(|(_, distance)| *distance > &2000.0)
                .for_each(|(idx, _)| mb.map.tiles[idx] = TileType::Wall);
        }
        mb.monster_spawns = mb.spawn_monsters(&center, rng);
        mb.player_start = center;
        mb.amulet_start = mb.find_most_distant();
        mb
    }
}
impl RandomWalkArchitect {
    fn walk(&mut self, start: &Point, rng: &mut RandomNumberGenerator, map: &mut Map) {
        let mut current_pos = start.clone();
        let mut distance_walked = 0;

        loop {
            let walk_idx = map.point2d_to_index(current_pos);
            map.tiles[walk_idx] = TileType::Floor;

            match rng.range(0, 4) {
                0 => current_pos.x -= 1,
                1 => current_pos.x += 1,
                2 => current_pos.y -= 1,
                _ => current_pos.y += 1,
            }

            if !map.in_bounds(current_pos) {
                break;
            }
            distance_walked += 1;
            if distance_walked > MAX_WALKING_DISTANCE {
                break;
            }
        }
    }
}
