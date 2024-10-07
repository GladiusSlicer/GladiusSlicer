use geo::*;

use geo_offset::*;

//todo remove dependency on geo clipper and by extension bindgen
//use geo_clipper::Clipper;

pub trait PolygonOperations {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64>;

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;
}

impl PolygonOperations for MultiPolygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        self.offset(delta).expect("All Points are valid")
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.difference(other)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.intersection(other)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.union(other)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.xor(other)
    }
}

impl PolygonOperations for Polygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        self.offset(delta).expect("All Points are valid")
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        MultiPolygon::from(self.clone()).difference(other)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        MultiPolygon::from(self.clone()).intersection(other)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        MultiPolygon::from(self.clone()).union(other)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        MultiPolygon::from(self.clone()).union(other)
    }
}
