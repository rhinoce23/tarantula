#include "polygon.h"

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
#pragma GCC diagnostic ignored "-Wunused-private-field"

#include "s2/s2point.h"
#include "s2/s2contains_point_query.h"
#include "s2/s2builderutil_snap_functions.h"
#include "s2/s2builderutil_s2polygon_layer.h"

#pragma GCC diagnostic pop


Polygons::Polygons() {
    polygonsIndex_ = std::make_unique<MutableS2ShapeIndex>();
}

Polygons::~Polygons() {
}

ErrorCode Polygons::add(Polygon polygon) {
    auto s2polygon = std::make_unique<S2Polygon>();
    std::vector<std::unique_ptr<S2Loop> > loops;
    for (auto a = polygon.loops_->begin(); a != polygon.loops_->end(); a++) {
        if (a->outer_) {
            a->loop_->Invert();
        }

        loops.push_back(std::move(a->loop_));
    }
    
    s2polygon->set_s2debug_override(S2Debug::DISABLE);
    s2polygon->InitNested(std::move(loops));
    polygonsIndex_->Add(absl::make_unique<S2Polygon::OwningShape>(std::move(s2polygon)));
    return ErrorCode::SUCCESS;
}

int Polygons::search(double lng, double lat) const {
    int r = -1;
    S2ContainsPointQueryOptions options(S2VertexModel::OPEN);       
    auto containsPointQuery = MakeS2ContainsPointQuery(polygonsIndex_.get(), options);  
    auto containsPointQueryResult = containsPointQuery.GetContainingShapes(
      S2Point(S2LatLng::FromDegrees(lat, lng)));
    if (!containsPointQueryResult.empty()) {
        r = containsPointQueryResult.front()->id();
    } 

    return r;
}

std::unique_ptr<SearchResult> Polygons::search_polygon(double lng, double lat) const {
    auto r = std::make_unique<SearchResult>();
    S2ContainsPointQueryOptions options(S2VertexModel::OPEN);       
    auto containsPointQuery = MakeS2ContainsPointQuery(polygonsIndex_.get(), options);  
    auto containsPointQueryResult = containsPointQuery.GetContainingShapes(
      S2Point(S2LatLng::FromDegrees(lat, lng)));
    if (!containsPointQueryResult.empty()) {
        r->index_ = containsPointQueryResult.front()->id();
        const S2Shape* shape = polygonsIndex_->shape(r->index_);
        auto poly_shape = dynamic_cast<const S2Polygon::Shape*>(shape);
        if (poly_shape) {
            auto polygon = poly_shape->polygon();

            for (int i = 0; i < polygon->num_loops(); ++i) {
                const S2Loop* loop = polygon->loop(i);
                for (int j = 0; j < loop->num_vertices(); ++j) {
                    S2LatLng latlng(loop->vertex(j));
                    r->lnglats_->push_back(LngLat(latlng.lng().degrees(), latlng.lat().degrees()));
                }
            }
        }
    } 

    return r;
}
