#version 450 core

layout(binding = 0) uniform DisplayUniforms {
    uvec2 bounds;
} display;

layout(push_constant) uniform SpriteUniforms {
    ivec2 pos;
    uvec2 bounds;
} sprite;

layout(location = 0) in vec2 point;

void main() {
    vec2 world_coords = sprite.pos + vec2(sprite.bounds) * point;

    gl_Position = vec4(
        2 * world_coords.x / display.bounds.x - 1, 
        2 * world_coords.y / display.bounds.y - 1, 
        0, 
        1
    );
}
