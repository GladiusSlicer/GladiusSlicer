use geo::{MultiPolygon, Polygon};

// todo remove dependency on geo clipper and by extension bindgen

pub trait PolygonOperations {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64>;

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64>;
}

impl PolygonOperations for MultiPolygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        geo_clipper::Clipper::offset(
            self,
            delta,
            geo_clipper::JoinType::Square,
            geo_clipper::EndType::ClosedPolygon,
            1_000_000.0,
        )
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::difference(self, other, 1_000_000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::intersection(self, other, 1_000_000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::union(self, other, 1_000_000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::xor(self, other, 1_000_000.0)
    }
}

impl PolygonOperations for Polygon<f64> {
    fn offset_from(&self, delta: f64) -> MultiPolygon<f64> {
        geo_clipper::Clipper::offset(
            self,
            delta,
            geo_clipper::JoinType::Square,
            geo_clipper::EndType::ClosedPolygon,
            1000000.0,
        )
    }

    fn difference_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::difference(self, other, 1_000_000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::intersection(self, other, 1_000_000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::union(self, other, 1_000_000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f64>) -> MultiPolygon<f64> {
        geo_clipper::Clipper::xor(self, other, 1_000_000.0)
    }
}
