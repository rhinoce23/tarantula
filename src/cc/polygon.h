#pragma once

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
#pragma GCC diagnostic ignored "-Wunused-private-field"
#pragma GCC diagnostic ignored "-Wdeprecated-builtins"
#pragma GCC diagnostic ignored "-Wsign-compare"
#pragma GCC diagnostic ignored "-Wunused-parameter"

#include "s2/s2polygon.h"
#include "s2/mutable_s2shape_index.h"

#pragma GCC diagnostic pop

#include <float.h>
#include <vector>
#include "error_codes.h"

class LngLat {
public:
    LngLat() : lng_(DBL_MAX), lat_(DBL_MAX) {}
    LngLat(double lng, double lat) : lng_(lng), lat_(lat) {}

    double lng() const {
        return lng_;
    }
    
    double lat() const {
        return lat_;
    }

    double lng_;
    double lat_;
};

class LngLats {
public:
    LngLats() : lnglats_(std::make_unique<std::vector<LngLat>>()) {}

    void add(double lng, double lat) {
        lnglats_->push_back(LngLat(lng, lat));
    }

    void pop_back() {
        if (lnglats_->size() > 3) {
            lnglats_->pop_back();
        }
    }   

    size_t size() const {
        return lnglats_->size();
    }

    std::unique_ptr<std::vector<LngLat>> lnglats_;
};

class Loop {
public:
    Loop() : loop_(std::make_unique<S2Loop>()), outer_(true) {}
    
    ErrorCode init(LngLats lnglats, bool outer, bool debug) {
        std::vector<S2Point> points;
        for (auto& lnglat : *lnglats.lnglats_) {
            points.push_back(S2Point(S2LatLng::FromDegrees(lnglat.lat_, lnglat.lng_)));
        }

        if (points.size() < 2) {
            return ErrorCode::TOO_FEW_VERTICES;
        }

        loop_->set_s2debug_override(S2Debug::DISABLE);
        loop_->Init(points);
        if (!loop_->IsValid()) {
            if (debug) {
                for (size_t i = 0; i < lnglats.lnglats_->size() - 1; i++) {
                    auto& lnglat = (*lnglats.lnglats_)[i];
                    auto& lnglat1 = (*lnglats.lnglats_)[i+1];
                    printf("%d %3.7f,%3.7f %3.7f,%3.7f\n", 
                        i, lnglat.lng_, lnglat.lat_, lnglat1.lng_, lnglat1.lat_);
                }
            }

            return ErrorCode::FAILURE;
        }

        if (outer && loop_->GetCurvature() > 0) {
            return ErrorCode::OUTER_CURVATURE;
        } else if (!outer && loop_->GetCurvature() < 0) {
            return ErrorCode::INNER_CURVATURE;
        }

        outer_ = outer;
   
        return ErrorCode::SUCCESS;
    }

    std::unique_ptr<S2Loop> loop_;
    bool outer_;
};

class Polygon {
public:
    Polygon() : loops_(std::make_unique<std::vector<Loop>>()){}

    void add(Loop loop) {
        loops_->push_back(std::move(loop));
    }

    std::unique_ptr<std::vector<Loop>> loops_;
};

class SearchResult {
public:
    SearchResult() : index_(-1), lnglats_(std::make_unique<std::vector<LngLat>>()) {}

    int index_;
    std::unique_ptr<std::vector<LngLat>>  lnglats_;

    int index() const {
        return index_;
    }

    std::unique_ptr<std::vector<LngLat>> lnglats() const{
        return std::make_unique<std::vector<LngLat>>(*lnglats_);
    }
};

class Polygons {
public:
    Polygons();
    ~Polygons();

    ErrorCode add(Polygon polgon);
    int search(double lng, double lat) const;
    std::unique_ptr<SearchResult> search_polygon(double lng, double lat) const;

protected:
    std::unique_ptr<MutableS2ShapeIndex> polygonsIndex_; // for rust, autocxx must use pointer!
};