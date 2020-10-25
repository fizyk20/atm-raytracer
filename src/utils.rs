use atm_refraction::EarthShape;
use nalgebra::Vector3;

pub fn world_directions(
    shape: &EarthShape,
    lat: f64,
    lon: f64,
) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
    match *shape {
        EarthShape::Flat => (
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ),
        EarthShape::Spherical { .. } => {
            let lat_rad = lat.to_radians();
            let lon_rad = lon.to_radians();

            let sinlon = lon_rad.sin();
            let coslon = lon_rad.cos();
            let sinlat = lat_rad.sin();
            let coslat = lat_rad.cos();

            // up direction
            let dirup = Vector3::new(coslat * coslon, coslat * sinlon, sinlat);
            // vector tangent to Earth's surface pointing north
            let dirn = Vector3::new(-sinlat * coslon, -sinlat * sinlon, coslat);
            // vector tangent to Earth's surface pointing east
            let dire = Vector3::new(-sinlon, coslon, 0.0);

            (dirn, dire, dirup)
        }
    }
}
