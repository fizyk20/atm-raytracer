use std::{path::PathBuf, str::FromStr};

use geotiff_rs::GeoTiff;
use lazy_static::lazy_static;
use regex::Regex;

use super::Tile;

pub struct GeoTiffWrapper {
    min_lat: f64,
    min_lon: f64,
    data: GeoTiff,
}

impl GeoTiffWrapper {
    pub fn coords_from_name(name: &PathBuf) -> Option<(i16, i16)> {
        lazy_static! {
            static ref RE: Regex = Regex::new("(N|S)(\\d+)(E|W)(\\d+)").unwrap();
        }
        let file_name = name.file_name()?.to_str()?;
        let cap = RE.captures_iter(file_name).next()?;
        let mut lat = i16::from_str(&cap[2]).ok()?;
        if &cap[1] == "S" {
            lat = -lat;
        }
        let mut lon = i16::from_str(&cap[4]).ok()?;
        if &cap[3] == "W" {
            lon = -lon;
        }
        Some((lat, lon))
    }

    pub fn from_path(name: &PathBuf) -> Option<Self> {
        let (lat, lon) = Self::coords_from_name(name)?;
        let data = GeoTiff::from_file(name).ok()?;
        Some(Self {
            min_lat: lat as f64,
            min_lon: lon as f64,
            data,
        })
    }
}

impl Tile for GeoTiffWrapper {
    fn min_latitude(&self) -> f64 {
        self.min_lat
    }

    fn max_latitude(&self) -> f64 {
        self.min_lat + 1.0
    }

    fn min_longitude(&self) -> f64 {
        self.min_lon
    }

    fn max_longitude(&self) -> f64 {
        self.min_lon + 1.0
    }

    fn get_elev(&self, lat: f64, lon: f64) -> Option<f64> {
        if lat < self.min_latitude()
            || lat > self.max_latitude()
            || lon < self.min_longitude()
            || lon > self.max_longitude()
        {
            return None;
        }
        let lat = (lat - self.min_lat) * 3600.0;
        let lon = (lon - self.min_lon) * 3600.0;

        let mut lat_int = lat as usize;
        let mut lon_int = lon as usize;

        let mut lat_frac = lat - lat_int as f64;
        let mut lon_frac = lon - lon_int as f64;

        // handle the edge case of max lat/lon
        if lat_int == 3600 {
            lat_int -= 1;
            lat_frac += 1.0;
        }
        if lon_int == 3600 {
            lon_int -= 1;
            lon_frac += 1.0;
        }

        // get values to interpolate
        let elev00 = self.data.get_pixel(lon_int, lat_int) as f64;
        let elev01 = self.data.get_pixel(lon_int, lat_int + 1) as f64;
        let elev10 = self.data.get_pixel(lon_int + 1, lat_int) as f64;
        let elev11 = self.data.get_pixel(lon_int + 1, lat_int + 1) as f64;

        let result = elev00 * (1.0 - lon_frac) * (1.0 - lat_frac)
            + elev01 * (1.0 - lon_frac) * lat_frac
            + elev10 * lon_frac * (1.0 - lat_frac)
            + elev11 * lon_frac * lat_frac;

        Some(result)
    }
}
