use geo::*;

//todo remove dependency on geo clipper and by extension bindgen
use geo_clipper::*;

pub trait PolygonOperations {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64>;

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;
}

impl PolygonOperations for MultiPolygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        self.offset(delta, JoinType::Square, EndType::ClosedPolygon, 100000.0)
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.difference(other, 100000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.intersection(other, 100000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.union(other, 100000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.union(other, 100000.0)
    }
}

impl PolygonOperations for Polygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        self.offset(delta, JoinType::Square, EndType::ClosedPolygon, 100000.0)
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.difference(other, 100000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.intersection(other, 100000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.union(other, 100000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        self.union(other, 100000.0)
    }
}
