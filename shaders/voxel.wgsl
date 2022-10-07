/// Represents a camera
struct Camera {
    position: vec3<f32>,
    look_at: vec3<f32>,
    zoom: f32,
};

/// The uniforms for the shader
struct Uniforms {
    time: f32,
    frames: u32,
    max_steps: u32,
    voxel_amount: u32,
    resolution: vec2<u32>,
    background_color: vec4<f32>,
    floor_color: vec4<f32>,
    object_color: vec3<f32>,
    light_position: vec3<f32>,
    sun_intensity: f32,
    smoothing: f32,
    ambient_occlusion: i32,
};

/// Represents a cast ray
struct Ray {
    position: vec3<f32>,
    direction: vec3<f32>,
};

/// Represetns a rayhit
struct RayHit {
    position: vec3<f32>,
    distance: f32,
    color: vec4<f32>,
};

/// Represents a voxel
struct Voxel {
    position: vec3<i32>,
    color: vec3<f32>,
};

// Shader uniforms
@group(0)
@binding(0)
var<uniform> uniforms: Uniforms;

// The camera
@group(0)
@binding(1)
var<uniform> camera: Camera;

// The voxel grid
@group(0)
@binding(2)
var<storage, read> voxels: array<Voxel>;

// The output texture
@group(0)
@binding(3)
var output: texture_storage_2d<rgba8unorm, write>;

/// Round the number to the nearest multiple
fn round_mul(num: f32, mul: i32) -> i32 {
    if (mul == 0) {
        return i32(num);
    }
    let remainder = abs(i32(num)) % mul;
    if (remainder == 0) {
        return i32(num);
    }
    if (i32(num) < 0) {
        return -(abs(i32(num)) - remainder);
    }
    return i32(num) + mul - remainder;
}

/// Calculates the smooth minimum between objects
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

/// Translate a pixel position to a grid position
fn get_grid_positon(position: vec3<f32>, grid_size: i32) -> vec3<i32> {
    let x = round_mul(position.x, grid_size);
    let y = round_mul(position.y, grid_size);
    let z = round_mul(position.z, grid_size);
    return vec3<i32>(x, y, z);
}

/// Create a ray cast from the camera
fn create_camera_ray(uv: vec2<f32>, camera: Camera) -> Ray {
    let f = normalize(camera.look_at - camera.position);
    let r = cross(vec3<f32>(0.0, 1.0, 0.0), f);
    let u = cross(f, r);
    
    let c = camera.position + f * (camera.zoom - 0.1);
    let i = c + uv.x * r + uv.y * u;
    let direction = i - camera.position;
    return Ray(camera.position, direction);
}

/// Draw a sphere
fn draw_sphere(
    hit: ptr<function, RayHit>, 
    raypos: vec3<f32>, 
    position: vec3<f32>, 
    radius: f32, 
    color: vec4<f32>
) {
    let distance= length(raypos - position) - radius; 
    // (*hit).distance = min((*hit).distance, distance);
    // Check if the current sphere is closer than the previous spheres
    if (distance < (*hit).distance) {
        // Update the hit
        (*hit).position = raypos;
        (*hit).color = color; 
        (*hit).distance = distance;
    }
}

/// Draw a rectangle
fn draw_rectangle(
    hit: ptr<function, RayHit>, 
    raypos: vec3<f32>, 
    position: vec3<f32>, 
    size: vec3<f32>, 
    color: vec4<f32>
) {
    let d = abs(raypos - position) - size;
    let distance = min(max(d.x, max(d.y, d.z)), 0.0) + length(max(d, vec3<f32>(0.0)));
    // (*hit).distance = min((*hit).distance, distance);
    // Check if the current sphere is closer than the previous spheres
    if (distance < (*hit).distance) {
        // Update the hit
        (*hit).position = raypos;
        (*hit).color = color; 
        (*hit).distance = distance;
    }
}

/// Draws the voxels
fn map(raypos: vec3<f32>, hit: RayHit) -> RayHit {
    var result = hit;
    // Draw each voxel
    for (var i = 0; i < (i32(uniforms.voxel_amount)); i += 1) {
        let voxel = voxels[i];
        draw_rectangle(
            &result, 
            raypos, 
            vec3<f32>(voxel.position) * 0.16 * 2.0, 
            vec3<f32>(0.16, 0.16, 0.16), 
            vec4<f32>(voxel.color, 1.0)
        );
    }
    return result;
}

/// Calculate the normals
fn calculate_normal(hit: RayHit) -> vec3<f32> {
    let step = 0.001;
    let raypos = hit.position;

    // Calculate the gradient
    let x_pl = map(vec3<f32>(raypos.x+step,raypos.y,raypos.z), hit).distance;
    let x_mi = map(vec3<f32>(raypos.x-step,raypos.y,raypos.z), hit).distance;
    let y_pl = map(vec3<f32>(raypos.x,raypos.y+step,raypos.z), hit).distance;
    let y_mi = map(vec3<f32>(raypos.x,raypos.y-step,raypos.z), hit).distance;
    let z_pl = map(vec3<f32>(raypos.x,raypos.y,raypos.z+step), hit).distance;
    let z_mi = map(vec3<f32>(raypos.x,raypos.y,raypos.z-step), hit).distance;
    let x = x_pl-x_mi;
    let y = y_pl-y_mi;
    let z = z_pl-z_mi;

    // Return the normalized gradient as a normal
    return normalize(vec3<f32>(x, y, z));
}

/// Calculate the ambient occlusion
fn ambient_occlusion(normal: vec3<f32>, step_dist: f32, steps: i32, hit: RayHit) -> f32 {
    var occlusion = 0.0;
    var max_occlusion = 0.0;

    for (var i = 0; i < steps; i += 1) {
        let p = hit.position + normal * (f32(i)+1.0) * step_dist;
        occlusion += 1.0 / pow(2.0, f32(i)) * map(p, hit).distance;
        max_occlusion += 1.0 / pow(2.0, f32(i)) * (f32(i)+1.0) * step_dist;
    }

    return occlusion / max_occlusion;
}


/// Calculates both the diffuse and ambient lighting
fn lighting(hit: RayHit) -> vec3<f32> {
    // Calculate diffuse lighting
    let normal = calculate_normal(hit);
    let light_dir = normalize(uniforms.light_position - hit.position);
    let diffuse = uniforms.sun_intensity / 10.0 + clamp(dot(normal, light_dir), 0.0, 1.0);
    // Return the ambient lighting
    // let ambient = ambient_occlusion(normal, 0.0015, uniforms.ambient_occlusion, hit);

    // Return the result
    return hit.color.xyz * diffuse; // * ambient; // color * diffuse * ambient;
}

// /// Draw a mandlebulb
// fn draw_mandlebulb(raypos: vec3<f32>, power: f32) -> f32 {
//     var z = raypos;
//     var dr = 1.0;
//     var r = 0.0;
//     
//     for (var i = 0; i < 32; i+=1) {
//     	r = length(z);
//     	if (r>10.0) {
//             break;
//         }
// 
//     	// convert to polar coordinates
//     	var theta = acos(z.z/r);
//     	var phi = atan2(z.y, z.x);
//     	dr = pow(r, power - 1.0) * power * dr + 1.0;
// 
//     	// scale and rotate the point
//     	var zr = pow(r, power);
//     	theta = theta * power;
//     	phi = phi * power;
// 
//     	// convert back to cartesian coordinates
//     	z = zr * vec3<f32>(f32(sin(theta) * cos(phi)), f32(sin(phi) * sin(theta)), f32(cos(theta)));
//     	z += raypos;
//     }
//     return 0.5*log(r)*r/dr;
// }

/// Cast a ray
fn cast_ray(ray: ptr<function, Ray>, id: vec3<u32>, max_iters: i32) -> RayHit {
    // The potential hit
    var hit: RayHit;
    hit.color = uniforms.background_color;
    hit.distance = f32(max_iters);
    hit = map((*ray).position, hit);

    // The distance travelled so far
    var travelled = hit.distance;
    
    for (var i = 0; i < max_iters && travelled > 0.01; i += 1) {
        // Calculate the ray's position
        let raypos = (*ray).position + travelled * (*ray).direction;
        // Update the potential hit
        hit = map(raypos, hit);
        // Check if the potential hit is close enough
        if (hit.distance < 0.001) {
            // Update the lighting
            hit.color = vec4<f32>(lighting(hit), 1.0);
            return hit;
        }   
        if (travelled > 1000.0) {
            break;
        }
        travelled += hit.distance;
    }

    hit.color = uniforms.background_color;
    return hit;
}

/// Shade a pixel
fn shade(id: vec3<u32>, uv: vec2<f32>, ray: ptr<function, Ray>) -> vec4<f32> {
    // Get the hit 
    let hit = cast_ray(ray, id, i32(uniforms.max_steps));
    // Return the hit color
    return hit.color;
}

/// """ 
/// Entrypoint
/// """
@compute
@workgroup_size(16, 16, 1)
fn main(
    @builtin(global_invocation_id) id: vec3<u32>,
) { 
    // Calculate the uv
    var uv = vec2<f32>(id.xy)/vec2<f32>(uniforms.resolution);
    // Offset so that the centre is the origin
    uv -= vec2<f32>(0.5);
    uv.y *= -1.0;
    // Create the ray
    var ray = create_camera_ray(uv, camera);
    // Shade the pixel
    textureStore(output, vec2<i32>(id.xy), shade(id, uv, &ray));
}