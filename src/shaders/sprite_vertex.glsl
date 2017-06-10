#version 450 core

layout(binding = 0) uniform DisplayUniforms {
    uvec2 bounds;
} display;

layout(location = 0) in vec2 point;

void main() {
    gl_Position = vec4(
        2 * point.x / display.bounds.x - 1, 
        2 * point.y / display.bounds.y - 1, 
        0, 
        1
    );
}
