pub fn experience_add(
    original_level: u32,
    original_exp: u32,
    exp_to_add: u32,
) -> (u32, u32) {
    // TODO a proper exp curve or table or whatever
    let new_level = original_level + ((original_exp + exp_to_add) / 100);
    let new_exp = (original_exp + exp_to_add) % 100;
    (new_level, new_exp)
}
