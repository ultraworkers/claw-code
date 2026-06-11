# 3D Mathematical Foundations for Game Development

**Source:** Core mathematical systems for 3D object manipulation
**Purpose:** Reference guide for AI agents generating 3D math code
**Prerequisites:** Linear Algebra, Trigonometry, Spatial Geometry

---

## 1. Coordinate Systems and Points

Before rendering a 3D object, you must define the space it exists in. A 3D space is defined by three intersecting axes: X, Y, and Z.

### Handedness

You must first choose a coordinate system "handedness." This determines the direction of the Z-axis (depth) relative to X (horizontal) and Y (vertical).

**Right-Handed System:**
- Point your right thumb along the +X axis and your index finger along the +Y axis, your middle finger points along the +Z axis (typically pointing *towards* the viewer)
- Used by: **OpenGL, Godot**

**Left-Handed System:**
- Using your left hand in the same way, the +Z axis points *away* from the viewer (into the screen)
- Used by: **DirectX, Unity, Unreal**

### Points vs. Vectors

In code, both points and vectors are stored as an array or struct of three floats (x, y, z). However, mathematically, they are distinct:

- **Point:** A specific location in space
- **Vector:** A direction and magnitude (length). It has no origin.

### Homogeneous Coordinates

To safely differentiate between them in mathematical operations, 3D math relies on Homogeneous Coordinates, which adds a fourth component, `w`.

- **For a Point:** `w = 1`. This allows the point to be moved (translated) in space.
- **For a Vector:** `w = 0`. This ensures that translating a direction has no effect (a direction pointing North is still pointing North no matter where you stand), but it can still be scaled or rotated.

---

## 2. Vectors: The Engine of Movement

Vectors are the primary tools for determining relationships between objects, calculating physics, and rendering lighting.

### Vector Normalization

A normalized vector (or unit vector) has a length (magnitude) of exactly 1. It represents pure direction.

To normalize a vector, you divide each of its components by its magnitude:

```
v_normalized = v / ||v||
```

Where `||v||` is the magnitude calculated using the Pythagorean theorem:

```
||v|| = sqrt(x² + y² + z²)
```

Normalizing vectors is computationally expensive because of the square root, so game engines often rely on fast inverse square root algorithms.

**Guardrail:** AI-generated code must never normalize a zero vector (causes division by zero). Always check `magnitude > 0.0001` before normalizing.

### The Dot Product

The Dot Product is arguably the most important mathematical operation in 3D graphics. It takes two vectors and returns a single scalar number.

```
a · b = ax*bx + ay*by + az*bz
```

Alternatively, it can be defined by the angle θ between them:

```
a · b = ||a|| * ||b|| * cos(θ)
```

**Game Design Applications:**

1. **Vision Cones:** If the dot product of an enemy's forward-facing vector and the vector pointing from the enemy to the player is positive, the player is in front of the enemy. If it's greater than 0.7 (roughly 45 degrees), the player is within a tight vision cone.

2. **Lighting:** Diffuse lighting (Lambertian reflectance) is calculated using the dot product of the light direction and the surface normal of the 3D object.

3. **Facing Detection:** `dot(forward, to_target) > 0` means the target is in front.

### The Cross Product

Unlike the dot product, the Cross Product of two vectors returns a **third vector** that is perfectly perpendicular (orthogonal) to both input vectors.

```
a × b = (ay*bz - az*by, az*bx - ax*bz, ax*by - ay*bx)
```

**Game Design Applications:**

1. **Surface Normals:** By taking the cross product of two edges of a triangle, you calculate the normal vector (the direction the triangle is facing).

2. **Camera Math:** To build a camera, you need a "Right" vector. By taking the cross product of the camera's "Forward" vector and the world's "Up" vector (0, 1, 0), you generate the camera's local X-axis.

3. **Torque/Rotation Direction:** Cross product determines the axis of rotation between two orientations.

**Guardrail:** Cross product order matters! `a × b = -(b × a)`. AI must use consistent handedness conventions.

---

## 3. Transformations: Matrix Mathematics

To move, rotate, and scale 3D objects, game engines use **Matrices**. A matrix is a grid of numbers. Because we use homogeneous coordinates (x, y, z, w), 3D game engines rely on **4×4 matrices**.

When you multiply a 3D vertex by a 4×4 matrix, that vertex is transformed.

### The Identity Matrix

The default state of an object is represented by the Identity Matrix (a diagonal line of 1s). Multiplying a vertex by the identity matrix leaves it completely unchanged.

```
| 1  0  0  0 |
| 0  1  0  0 |
| 0  0  1  0 |
| 0  0  0  1 |
```

### Translation Matrix

Modifies the right-most column of the 4×4 matrix. Because a point has w=1, the matrix multiplication adds these translation values to the x, y, z coordinates.

```
| 1  0  0  tx |
| 0  1  0  ty |
| 0  0  1  tz |
| 0  0  0  1  |
```

### Scaling Matrix

Modifies the diagonal values of the matrix to stretch or shrink the object along specific axes.

```
| sx  0   0   0 |
| 0   sy  0   0 |
| 0   0   sz  0 |
| 0   0   0   1 |
```

### Rotation Matrix

Uses sine and cosine functions to orbit vertices around an axis. Example: rotating around the Z-axis:

```
| cos(θ)  -sin(θ)  0   0 |
| sin(θ)   cos(θ)  0   0 |
| 0        0       1   0 |
| 0        0       0   1 |
```

### Matrix Concatenation

The true power of matrices is that you can multiply them together to combine transformations into a single 4×4 matrix.

If you want to scale an object, then rotate it, then translate it:

```
M_final = M_translation * M_rotation * M_scale
```

**Note:** Matrix multiplication is **not commutative**: `A · B ≠ B · A`. The order of operations is critical. In game engines, you typically **scale first, rotate second, and translate last** (SRT order).

**Guardrail:** AI-generated code must preserve SRT order. Non-uniform scale before rotation causes shearing.

---

## 4. Quaternions vs. Euler Angles

When designing 3D rotation math, your first instinct will be to store rotation as three simple angles: pitch (X), yaw (Y), and roll (Z). These are called **Euler Angles**.

### The Gimbal Lock Problem

Euler angles are evaluated sequentially. If an object pitches 90 degrees up, the X-axis aligns perfectly with the Z-axis. Suddenly, changing the "yaw" does the exact same thing as changing the "roll". You have lost a degree of freedom. This is known as **Gimbal Lock**, and it causes objects to spin wildly or freeze when looking straight up or down.

### The Quaternion Solution

To solve Gimbal Lock, 3D engines use **Quaternions**. A quaternion is a complex mathematical concept based on a 4D hyper-sphere, represented by four components:

```
q = (x, y, z, w)
```

Instead of rotating sequentially around three axes, a quaternion defines a single, arbitrary axis in 3D space and an angle to rotate around that specific axis.

**Why you must implement Quaternions:**

1. **No Gimbal Lock:** Because rotation happens in a single step around a custom axis, axes never align and lock.

2. **Smooth Interpolation:** Quaternions allow for **SLERP** (Spherical Linear Interpolation). If you want an AI guard to smoothly turn its head from looking at point A to point B, SLERP calculates the shortest, smoothest rotational arc between those two orientations at a constant velocity.

**Guardrail:** All internal object rotations must be stored as quaternions, only converting them to matrices at the very end of the rendering pipeline. AI must never store persistent rotation state as Euler angles.

---

## 5. The Rendering Pipeline: The MVP Matrix

To get a 3D object onto a 2D screen, every single vertex of that object must pass through a strict mathematical pipeline known as the **MVP (Model-View-Projection) Matrix**.

### 1. The Model Matrix (Local → World Space)

When an artist creates a 3D object (like a car), the vertices are centered around a local origin (0,0,0). The Model matrix applies the object's position, rotation, and scale in the game world, transforming those vertices from **Local Space** into **World Space**.

### 2. The View Matrix (World → Camera Space)

The game camera does not actually exist. To simulate a camera, the entire 3D world is moved in the exact opposite direction of the camera. If the camera moves 5 units forward, the math actually moves the entire world 5 units backward. The View matrix transforms vertices from **World Space** into **Camera Space**.

### 3. The Projection Matrix (Camera → Clip Space)

This is where 3D becomes 2D. The projection matrix defines the camera's **Frustum** (a truncated pyramid representing what the camera can see). It applies perspective, where objects further away appear smaller.

This is achieved mathematically via the **Perspective Divide**. The matrix modifies the vertex's `w` component based on its Z-depth. By dividing the x, y, and z coordinates by `w`, distant objects are squeezed toward the center of the screen.

The resulting coordinates are **Normalized Device Coordinates (NDC)** ranging from -1 to 1, which the GPU then maps to the actual pixel dimensions of the player's monitor.

**Guardrail:** AI-generated projection matrices must handle near-plane clipping correctly. A near-plane of 0.0 causes division-by-zero in perspective divide.

---

## 6. Physics and Spatial Mathematics

Rendering the object is only half the battle. Your math architecture must also support physical interactions.

### AABB (Axis-Aligned Bounding Box)

The simplest 3D collision volume is an AABB. It is defined simply by a `Min(x,y,z)` and a `Max(x,y,z)` point.

To check if two objects are colliding:

```
if (a.max.x < b.min.x or a.min.x > b.max.x) return false;
if (a.max.y < b.min.y or a.min.y > b.max.y) return false;
if (a.max.z < b.min.z or a.min.z > b.max.z) return false;
return true; // Collision occurred!
```

However, "Axis-Aligned" means the box cannot rotate. If your 3D car rotates 45 degrees, the AABB must expand to encompass the new dimensions, making collisions less accurate.

**Guardrail:** AI must prefer OBB (Oriented Bounding Box) or primitive colliders for rotating objects. AABB-only collision for dynamic objects is a performance/accuracy tradeoff that must be explicitly documented.

### Raycasting and Barycentric Coordinates

Shooting a gun, clicking on a 3D object with a mouse, or calculating line-of-sight requires **Raycasting**. A ray is defined by:

- Origin point **P**
- Normalized direction vector **D**

A point on the ray at distance `t`:

```
Point = P + D * t
```

**Barycentric Coordinates** are a coordinate system used to define a point precisely within the bounds of a triangle's three vertices. This allows game engines to know exactly where to spawn a bullet hole decal on a complex 3D mesh.

**Guardrail:** AI-generated raycast code must include `t > 0` checks to prevent detecting collisions behind the ray origin. Max ray distance must be specified to avoid infinite searches.

---

## 7. Floating-Point Precision and World-Space Limits

3D engines using 32-bit floats lose precision at large world coordinates. At 10,000 units from origin, position precision drops to ~1mm. At 100,000 units, it's ~1cm — enough to cause visible jitter.

**Guardrail - PRECISION-01:** AI must implement floating-origin systems for open-world games. Periodically re-center the world around the player, shifting all object positions by the same offset.

**Guardrail - PRECISION-02:** Never place gameplay-critical precision requirements (sniper scopes, fine platforming) beyond 10km from world origin without double-precision positioning or floating-origin.

**Guardrail - PRECISION-03:** Camera far/near plane ratio should not exceed 10,000:1 to prevent Z-fighting. For large draw distances, use logarithmic depth buffering.

---

## Conclusion

Designing math for 3D games is an exercise in managing illusions. You are building spatial frameworks, manipulating homogeneous coordinates to slide objects through space, leveraging quaternions to avoid catastrophic physics locks, and dividing by depth to mimic human perspective.

A strong architecture relies on:
1. Highly optimized vector math libraries
2. Strict separation of spatial coordinates from rendering logic
3. Consistent coordinate system conventions throughout the pipeline
4. Quaternions for all persistent rotation state
5. Fixed-point or double-precision for large-world scenarios

---

*Part of Agent Guardrails Template v3.1.0 — 3D Mathematical Foundations*
