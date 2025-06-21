# tarantula
대용량 요청 트래픽 처리를 위한 Reverse Geocoding 서버 구현: 공공 데이터 활용과 필요한 기술 스택 제시

## Abstract
모빌리티 서비스를 만들면 현재 위치의 주소를 표시해 주는 UX 가 많이 들어간다.  

주소를 구성하는 계층 폴리곤들은 국가에서 관리하는 공공 데이타이다.  

위치로 주소를 얻을 수 있는 reverse geocoding API 는 서비스에서 사용할 수준의 무료 공공 API 찾기는 쉽지 않다.  
구글, 네이버, 카카오 등의 포털 위치 서비스 회사의 유료 API 를 사용해야 한다.  

공공 데이타를 활용해서 구현할 수 있는 적절한 기술 스택을 제시한다. 

## Introduction
한 점이 폴리곤에 포함 되는지를 판단하는건 "ray casting algorithm" 이다.  

mysql, postgres 같은 RDB 의 geometry 연산을 이용하려고 테이블에 넣고 쿼리로 하면 쉽지 않을까 ?  
쉽다, 하지만 느린 응답 속도 특히 파일이 메모리 캐쉬로 전환되는 시점에서 서버는 폭주하는 경우가 발생한다.   
그렇다고 파일 보다 큰 메모리를 넣어서 전체 메모리 캐쉬에 올리고 사용하는건 굳이 RDB 를 사용하는 이유가 없다.  

원본 파일 shapefile 을 로딩해서 spatial index 을 메모리에 로딩해서 서버로 서빙한다.   
많은 개발 항목들이 있을거 같지만 그렇지 않다 25년 현재 너무 좋은 오픈 소스들이 많다.  

공공 데이타로 다운받은 shapfile 을 사용해서 주소 지번 폴리곤까지 검색하는 서버를 구현한다.  
적절한 언어, geometry 라이브러리도 제시한다
컨버터, 폴리곤 편집툴도 구현한다.  

데이타를 공개할때 몇가지 고려할 사항을 제시한다.

## Implementation
LLM 없었다면 구현하기가 쉽지 않았을거다.  

### Tech Stack
#### rust "server"
자바/코트린, 파이선, node(javascript), ... 로 서버를 쉽게 구현한다.  
포털 수준의 트래픽을 감당하려면 굉장히 많은 리소스가 든다.  
트패픽이 많은 부분은 apapce + c/c++ module, erlang, 같은 언어로 개발한다.   
golang 으로 개발을 하는 곳도 많다 러닝커브는 낮고 자바 보다 성능이 좋기 때문이다.   

뛰어난 성능과 안정성적으로 서버를 구현할 수 있는 언어는 무었일까?   
컴파일만 되면 컴파일 타임 메모리 안전성, 가비지 컬렉션 불필요, 스레드 안전성 보장, ... 너무 좋다.   
러닝 커브가 많지만 LLM 을 이용하니 그리 부담은 되지 않았다.  

지역별 계층 폴리곤을 par_iter() 로 병렬 로딩하는 코드 예
```
config.districts
  .par_iter()
  .try_for_each(|district| {
    config.district_par
      .par_iter()
      .try_for_each(|name|
        self.load_district_par_polygons(
          config,
          district, 
          name,
          config.debug, 
          &config.debug_name,
        )
      )?;
    config.district_par_any
      .par_iter()
      .try_for_each(|name|
        self.load_district_par_any_polygons(
          config,
          district, 
          name,
          config.debug, 
          &config.debug_name,
        )
      )
    })?;
```

#### s2geometry "geometry spatial index"
Eric Veach 가 구글 Maps 에 들어와서 만들었다.  

**S2 is a library for spherical geometry that aims to have the same robustness,  
flexibility, and performance as the very best planar geometry libraries.**   

#### autocxx "rust binding c++"
s2geometry 가 rust 로 전체가 포팅되어 있지 않다.  
rust 에서 c++ 를 안전하게 사용하기 위해 autocxx 를 사용한다.  

#### config "toml"
설정 파일을 프로그램을 자유롭게 동작하기 위해 매우 중요하다.   
xml, json, yml ... 많은 것들이 있다.   
그 중 최악은 역시 xml 인것 같다.   

config.toml 설정 파일에서 데이타 로딩, 검색 방법을 정의한다.   
```
[search.shapefile]
path = "./data/converted"

[search.shapefile.attributes]
"TL_SCCO_CTPRVN" = { level = 1, names =  ["CTPRVN_CD", "CTP_KOR_NM", "CTP_ENG_NM"]}
"TL_SCCO_SIG" = { level = 2, names = ["SIG_CD", "SIG_KOR_NM", "SIG_ENG_NM"]}
"TL_SCCO_EMD" = { level = 3, names = ["EMD_CD", "EMD_KOR_NM", "EMD_ENG_NM"]}
"TL_SCCO_LI" = { level = 4, names = ["LI_CD", "LI_KOR_NM", "LI_ENG_NM"]}
"AL_D002_" = { level = 5, names = ["A3", "A4", "A5"]}

[search]
districts = [
  #"11000",    
  #"41000",    
  #"46000", 
  "36000",   
]

hierarchies = [
  "TL_SCCO_CTPRVN",
]

district_par = [
  "TL_SCCO_SIG",
  "TL_SCCO_EMD",
  "TL_SCCO_LI",
]

district_par_any = [
  "AL_D002_"
]

[rest]
port = 8080
host = "localhost"

[grpc]
port = 8090
host = "localhost"
```
#### electron "editing geometry"
오류 폴리곤을 편집할 필요가 있다.   
UI 프로그램은 별도의 uikit 없이 웹 html 로 하는게 가장 효율적이다.   
shapefile 을 로딩해서 폴리곤을 수정하고 저장한다.   

웹 기술(HTML, CSS, JavaScript)을 사용하여 데스크톱 애플리케이션을 만들 수 있게 해주는 오픈 소스 프레임워크다.   
간단히 말해, 웹 개발자가 웹 개발 경험과 기술을 활용하여   
Windows, macOS, Linux와 같은 다양한 운영체제에서 실행되는 네이티브 데스크톱 애플리케이션을 구축할 수 있도록 도와주는 도구다.   
LLM 으로 바이브 코딩해서 개발한다.  

#### postman "rest, grpc api test"
rest, grpc 를 test 할때 사용하는 도구이다   

### Data
국가는 대한민국으로 한다.   
행정구역은 계층을 가진다 ex) 경기도(시도) > 용인시 수지구(시군구) > 신봉동(읍면동) > 신봉1로 214   
국가를 level0, 경기도 level1, ... 로 정의 한다   

행정구역 폴리곤은 "주소기반산업지원서비스(제공하는 주소)" 에서 level1 단위로 다운로드 할 수 있다   
지번은 "v-world 디지털 국토 트윈 > 연속지적도형정보" 에서 level1 단위로 다운로드 할 수 있다   

#### 행정구역 폴리곤
주소기반산업지원서비스(제공하는 주소), 좌표계(ITRF2000), 기준타원체(GRS80), 투영법(UTM), 장반경(6,378,137m)   
https://business.juso.go.kr/addrlink/adresInfoProvd/guidance/provdAdresInfo.do    
전자지도 다운로드 > 도로명주소 전자지도   

- 세종특별시 36000.zip   
```
시도 TL_SCCO_CTPRVN.shp 
시군구 TL_SCCO_SIG.shp 
읍면도 TL_SCCO_EMD.shp 
리 TL_SCCO_LI.shp 
```

#### 지번, premise 폴리곤  
연속지적도형정보, EPSG:5186(GRS80)  
https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?svcCde=NA&dsId=23   

- 세종특별시   
```
AL_D002_36_20250504.zip
```

#### 이슈들
원본 데이타는 다음과 같은 작업하기 불편한 이슈들이 있다.   
- euc_kr 문자열 인코딩
- 통일되지 않은 좌표계
- 파일 크기  

##### 원본 컨버젼  
convert 를 실행하면 euc_kr, 5179 좌표계를 utf8, 4326
파일 크기를 12MB 가 넘으면 60만개 보간점 한계로 파일를 분리한다.
```
go run convert.go -h
../data/source/46000/AL_D002_46_20250204.shp -> ../data/all_converted/46000/AL_D002_46_20250204_part1.shp
../data/all_converted/46000/AL_D002_46_20250204_part2.shp
../data/all_converted/46000/AL_D002_46_20250204_part3.shp
...
../data/all_converted/46000/AL_D002_46_20250204_part24.shp
../data/all_converted/46000/AL_D002_46_20250204_part25.shp
../data/source/46000/TL_SCCO_CTPRVN.shp -> ../data/all_converted/46000/TL_SCCO_CTPRVN.shp
```
##### s2gemetry polygon validation  
s2geometry 를 사용하기에 로딩할 수 없는 polygon 을 누락 시킨다.   
아래 같은 오류들이 나온다.     
- 중복 포인트, ERROR Edge 1990 is degenerate (duplicate vertex)
- 서로 겹치는 선분, ERROR Edge 455 crosses edge 457
- outer, inner curvature 
- empty polygon
``` 
cargo test -- --nocapture test_search_polygon   
cargo test -- --nocapture test_load_polygon   
```

##### 오류 폴리곤 편집
오류 폴리곤을 편집할 수 있는 도구를 만들었다.   
아래 처럼 실행해서 업데이트 할 수 있다.   
```
npm run start -- --shapefile-path ../data/converted/46000/TL_SCCO_EMD.shp    
npm run start -- --shapefile-path ../data/all_converted/46000/AL_D002_46_20250204_6_part22.shp 
'u' 업데이트, 'x' 선택 취소, 's' 저장
```

아래 처럼 다양한 원본 데이타 오류가 있다  
<img width="320" alt="image" src="https://github.com/user-attachments/assets/4f15da3f-418b-4a1a-b336-f4d81a1da3ca" />
<img width="320" alt="image" src="https://github.com/user-attachments/assets/453193a6-2774-431c-8066-fde5e755e8f9" />
<img width="320" alt="image" src="https://github.com/user-attachments/assets/b0a97660-e15a-4de2-97ce-762eff70be32" />
<img width="320" alt="image" src="https://github.com/user-attachments/assets/e69b592d-acb3-4777-8eac-4ab608588038" />
 
### 실행
```
cargo run
    Running `target/debug/tarantula`
load_district_polygons 36000 TL_SCCO_SIG elapsed: 129.599833ms
load_district_polygons 36000 TL_SCCO_EMD elapsed: 297.3095ms
load_district_polygons 36000 TL_SCCO_LI elapsed: 311.834583ms
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 0 crosses edge 2
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 5-2 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 18 crosses edge 21
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 38-4 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 5 crosses edge 21
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 81-1 error_code: 1
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 138-2 error_code: 3
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 0 crosses edge 3
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part2.shp load_polygon 524 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 767 crosses edge 770
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part3.shp load_polygon 24-1 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 4 crosses edge 8
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part3.shp load_polygon 134-1 error_code: 1
load_district_par_any_polygons 36000 AL_D002_ elapsed: 9.017407292s
Search load elapsed: 9.467750875s
grpc server listening on [::1]:8090
rest api server listening on [::1]:8080
```
- rest api
<img width="820" alt="image" src="https://github.com/user-attachments/assets/323f435f-dd47-4d9d-add7-0b53ec4b7a4f" />  

- grpc api
<img width="820" alt="image" src="https://github.com/user-attachments/assets/2f268f2d-e2c6-4d6b-9c75-72e1b78df5f6" />

#### 성능
- rest api
apple m1 pro 32G
```
ab -n 1000000 -c 60 "http://localhost:8080/tarantula?lon=126.973826366&lat=37.532190912"
Server Software:
Server Hostname:        localhost
Server Port:            8080

Document Path:          /tarantula?lon=126.973826366&lat=37.532190912
Document Length:        208 bytes

Concurrency Level:      60
Time taken for tests:   59.478 seconds
Complete requests:      1000000
Failed requests:        0
Total transferred:      317000000 bytes
HTML transferred:       208000000 bytes
Requests per second:    16812.83 [#/sec] (mean)
Time per request:       3.569 [ms] (mean)
Time per request:       0.059 [ms] (mean, across all concurrent requests)
Transfer rate:          5204.75 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.8      0     116
Processing:     0    3   3.6      3     125
Waiting:        0    3   3.5      3     125
Total:          0    4   3.6      3     125
```
- grpc api
```
bash pref_grpc.sh pref_grpc_req.json
```

## Conclusion
### 정부가 공공데이타나 api 를 공개할때 고려할 사항
- 데이터 품질과 정확성 포맷
오류나 불완전한 데이터는 사용자에게 혼란을 줄 수 있습니다  
문자열 인코딩은 utf8, 좌표계 EPSG:4326 로 특이한 사항이 없으면 통일 한다   
2차 가공이 필요없게 서비스에 사용 가능한지 테스트도 해야 한다  
- API 기술적 안정성
API의 가용성과 안정성을 보장하기 위해 서버 용량, 트래픽 관리, 장애 대응 계획을 준비해야 한다  
이 부분이 지속 가능성에 무리가 있으면 데이타를 공개 한다  
- 활용도와 공공성
공공데이터의 목적이 국민의 편익 증진과 공공 문제 해결에 있으므로, 사회적·경제적 가치를 창출할 수 있는 데이터를 우선 공개한다  
다양한 분야(예: 교통, 의료, 환경)에서 활용 가능하도록 데이터 범위를 고려한다  
- 사용자 피드백과 개선
공개 후 사용자 의견을 수집해 데이터 품질, API 기능, 문서 등을 지속적으로 개선한다  

### 기술 스택
- 서버는 rust 언어
- geometry 엔진은 s2geometry c++, autocxx
- 디버깅 웹은 electron cross-platform web app 

## Quick Start
https://www.youtube.com/watch?v=zJDaM0ofj6Y 
- 원본 데이타 압축 풀기
세종특별시 데이타 data/source/36000.zip (행정계), AL_D002_36_20250504.zip (지번) 가 있고  
data/source/36000 폴더에 압축을 푼다  
```
bash data.sh
```
- 한글 인코딩, 좌표계, 파일 크기 분할 컨버젼
data/converted 폴더에 컨버젼 한다
```
cd shape_convert
go run convert.go
```
- 서버 실행
```
cargo run
```
- 디버깅 웹 실행
```
cd shape_edit
npm install --save-dev electron
npm run start
```

