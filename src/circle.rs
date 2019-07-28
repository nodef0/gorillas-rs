use quicksilver::{
    geom::{about_equal, Circle, Rectangle, Scalar, Shape, Transform, Vector},
    graphics::{GpuTriangle, Mesh, Drawable, Background}
};
use std::iter;
use std::{
    cmp::{Eq, PartialEq},
};

#[derive(Clone, Copy, Default, Debug)]
pub struct CircleF {
    pub pos: Vector,
    pub radius: f32,
}

impl PartialEq for CircleF {
    fn eq(&self, other: &CircleF) -> bool {
        return about_equal(self.pos.x, other.pos.x)
            && about_equal(self.pos.y, other.pos.y)
            && about_equal(self.radius, other.radius)
    }
}

impl Eq for CircleF {}

impl Shape for CircleF {
    fn contains(&self, v: impl Into<Vector>) -> bool {
        (v.into() - self.center()).len2() < self.radius.powi(2)
    }
    fn overlaps_circle(&self, c: &Circle) -> bool { 
        (self.center() - c.center()).len2() < (self.radius + c.radius).powi(2)
    }
    fn overlaps(&self, shape: &impl Shape) -> bool {
        shape.overlaps_circle(&Circle {
            pos: self.pos,
            radius: self.radius
        })
    }

    fn center(&self) -> Vector { self.pos }
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.pos - Vector::ONE * self.radius, Vector::ONE * 2 * self.radius)
    }
    fn translate(&self, v: impl Into<Vector>) -> Self {
        CircleF {
            pos: self.pos + v.into(),
            radius: self.radius
        }
    }
}

impl Drawable for CircleF {
    fn draw<'a>(&self, mesh: &mut Mesh, bkg: Background<'a>, trans: Transform, z: impl Scalar) {
        let trans = Transform::translate(self.center())
            * trans
            * Transform::scale(Vector::ONE * self.radius);
        let tex_trans = bkg.image().map(|img| img.projection(Rectangle::new((-1,-1), (2,2))));
        let offset = mesh.add_positioned_vertices(CIRCLE_POINTS.iter().cloned(), trans, tex_trans, bkg);
        mesh.triangles.extend(iter::repeat(z)
            .take(CIRCLE_POINTS.len() - 1)
            .enumerate()
            .map(|(index, z)| GpuTriangle::new(offset, [0, index as u32, index as u32 + 1], z, bkg)));
    }
}

const CIRCLE_POINTS: [Vector; 64] = [
    Vector { x: 1.0, y: 0.0 },
    Vector { x: 0.9950307753654014, y: 0.09956784659581666 },
    Vector { x: 0.9801724878485438, y: 0.19814614319939758 },
    Vector { x: 0.9555728057861407, y: 0.2947551744109042 },
    Vector { x: 0.9214762118704076, y: 0.38843479627469474 },
    Vector { x: 0.8782215733702285, y: 0.47825397862131824 },
    Vector { x: 0.8262387743159949, y: 0.5633200580636221 },
    Vector { x: 0.766044443118978, y: 0.6427876096865394 },
    Vector { x: 0.6982368180860729, y: 0.7158668492597184 },
    Vector { x: 0.6234898018587336, y: 0.7818314824680298 },
    Vector { x: 0.5425462638657594, y: 0.8400259231507714 },
    Vector { x: 0.4562106573531629, y: 0.8898718088114687 },
    Vector { x: 0.365341024366395, y: 0.9308737486442042 },
    Vector { x: 0.27084046814300516, y: 0.962624246950012 },
    Vector { x: 0.17364817766693022, y: 0.9848077530122081 },
    Vector { x: 0.07473009358642417, y: 0.9972037971811801 },
    Vector { x: -0.024930691738072913, y: 0.9996891820008162 },
    Vector { x: -0.12434370464748516, y: 0.9922392066001721 },
    Vector { x: -0.22252093395631434, y: 0.9749279121818236 },
    Vector { x: -0.31848665025168454, y: 0.9479273461671317 },
    Vector { x: -0.41128710313061156, y: 0.9115058523116731 },
    Vector { x: -0.5000000000000002, y: 0.8660254037844385 },
    Vector { x: -0.58374367223479, y: 0.8119380057158564 },
    Vector { x: -0.6616858375968595, y: 0.7497812029677341 },
    Vector { x: -0.7330518718298263, y: 0.6801727377709194 },
    Vector { x: -0.7971325072229225, y: 0.6038044103254774 },
    Vector { x: -0.8532908816321556, y: 0.5214352033794981 },
    Vector { x: -0.900968867902419, y: 0.43388373911755823 },
    Vector { x: -0.9396926207859084, y: 0.3420201433256685 },
    Vector { x: -0.969077286229078, y: 0.24675739769029342 },
    Vector { x: -0.9888308262251285, y: 0.14904226617617428 },
    Vector { x: -0.9987569212189223, y: 0.04984588566069704 },
    Vector { x: -0.9987569212189223, y: -0.04984588566069723 },
    Vector { x: -0.9888308262251285, y: -0.14904226617617447 },
    Vector { x: -0.969077286229078, y: -0.24675739769029362 },
    Vector { x: -0.9396926207859084, y: -0.34202014332566866 },
    Vector { x: -0.9009688679024191, y: -0.433883739117558 },
    Vector { x: -0.8532908816321555, y: -0.5214352033794983 },
    Vector { x: -0.7971325072229224, y: -0.6038044103254775 },
    Vector { x: -0.7330518718298262, y: -0.6801727377709195 },
    Vector { x: -0.6616858375968594, y: -0.7497812029677342 },
    Vector { x: -0.5837436722347898, y: -0.8119380057158565 },
    Vector { x: -0.4999999999999996, y: -0.8660254037844388 },
    Vector { x: -0.4112871031306116, y: -0.9115058523116731 },
    Vector { x: -0.3184866502516841, y: -0.9479273461671318 },
    Vector { x: -0.2225209339563146, y: -0.9749279121818236 },
    Vector { x: -0.12434370464748495, y: -0.9922392066001721 },
    Vector { x: -0.024930691738073156, y: -0.9996891820008162 },
    Vector { x: 0.07473009358642436, y: -0.9972037971811801 },
    Vector { x: 0.17364817766693083, y: -0.984807753012208 },
    Vector { x: 0.2708404681430051, y: -0.962624246950012 },
    Vector { x: 0.3653410243663954, y: -0.9308737486442041 },
    Vector { x: 0.45621065735316285, y: -0.8898718088114687 },
    Vector { x: 0.5425462638657597, y: -0.8400259231507713 },
    Vector { x: 0.6234898018587334, y: -0.7818314824680299 },
    Vector { x: 0.698236818086073, y: -0.7158668492597183 },
    Vector { x: 0.7660444431189785, y: -0.6427876096865389 },
    Vector { x: 0.8262387743159949, y: -0.563320058063622 },
    Vector { x: 0.8782215733702288, y: -0.4782539786213178 },
    Vector { x: 0.9214762118704076, y: -0.38843479627469474 },
    Vector { x: 0.9555728057861408, y: -0.2947551744109039 },
    Vector { x: 0.9801724878485438, y: -0.19814614319939772 },
    Vector { x: 0.9950307753654014, y: -0.09956784659581641 },
    Vector { x: 1.0, y: 0.0 },
];

