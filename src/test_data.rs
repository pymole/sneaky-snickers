/*
 turn=200
 A: health=100
 B: health=100
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . B . . . . .
 . . a a A b . . . . .
 . . . . . b . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const BODY_COLLISION: &str = include_str!("test_data/body_collision.json");

/*
 turn=200
 A: health=100
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . a a . . . . . . . .
 A a . . . . . . . . .
*/
pub const BOTTOM_LEFT_CORNER: &str = include_str!("test_data/bottom_left_corner.json");

/*
 turn=200
 A: health=100
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . a a a . . . . .
 . . . a . a . . . . .
 . . . a A a . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const FOLLOW_TAIL: &str = include_str!("test_data/follow_tail.json");

/*
 turn=202
 A: health=99
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . ¤ . A . . . .
 . . . . . . a . . . .
 . . . . . . a . ¤ . .
 . . . . . a a . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const FOOD_IN_FRONT_UP_UP: &str = include_str!("test_data/food_in_front_up_up.json");

/*
 turn=200
 A: health=21
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . ¤ . . . . . .
 . . . . . . ¤ . . . .
 . . . . . . A . ¤ . .
 . . . . a a a . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const FOOD_IN_FRONT: &str = include_str!("test_data/food_in_front.json");

/*
 turn=201
 A: health=99
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . A a . . . . .
 . . . . . a . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const SINGLE_SHORT_SNAKE_IN_THE_CENTER_LEFT: &str = include_str!("test_data/single_short_snake_in_the_center_left.json");

/*
 turn=201
 A: health=99
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . a A . . . .
 . . . . . a . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const SINGLE_SHORT_SNAKE_IN_THE_CENTER_RIGHT: &str = include_str!("test_data/single_short_snake_in_the_center_right.json");

/*
 turn=201
 A: health=99
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . A . . . . .
 . . . . . a . . . . .
 . . . . . a . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const SINGLE_SHORT_SNAKE_IN_THE_CENTER_UP: &str = include_str!("test_data/single_short_snake_in_the_center_up.json");

/*
 turn=200
 A: health=100
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . A . . . . .
 . . . . . a . . . . .
 . . . . . a . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const SINGLE_SHORT_SNAKE_IN_THE_CENTER: &str = include_str!("test_data/single_short_snake_in_the_center.json");

/*
 turn=200
 A: health=100
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . a a a . . . . .
 . . . a . a . . . . .
 . . . a A a . . . . .
 . . . a . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const STEP_ON_TAIL: &str = include_str!("test_data/step_on_tail.json");

/*
 turn=200
 A: health=100
 . . . . . . . . a a A
 . . . . . . . . a . .
 . . . . . . . . a . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
 . . . . . . . . . . .
*/
pub const TOP_RIGHT_CORNER: &str = include_str!("test_data/top_right_corner.json");

pub const HEAD_TO_HEAD_BIG_AND_SMALL: &str = include_str!("test_data/head_to_head_big_and_small.json");

pub const FOOD_HEAD_TO_HEAD_EQUAL: &str = include_str!("test_data/food_head_to_head_equal.json");

pub const FOOD_HEAD_TO_HEAD_EQUAL_V2: &str = include_str!("test_data/food_head_to_head_equal_v2.json");
