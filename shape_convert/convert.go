package main

import (
	"bytes"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"

	"github.com/jonas-p/go-shp"
	"github.com/pebbe/proj/v5"
	"github.com/pkg/errors"
	"github.com/samber/lo"
	"golang.org/x/text/encoding/korean"
	"golang.org/x/text/transform"
)

const (
	EPSG5179PROJ = "+proj=tmerc +lat_0=38 +lon_0=127.5 +k=0.9996 +x_0=1000000 +y_0=2000000 +ellps=GRS80 +units=m +no_defs"
	EPSG4326PROJ = "+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs"
	EPSG5181PROJ = "+proj=tmerc +lat_0=38 +lon_0=127 +k=1 +x_0=200000 +y_0=500000 +ellps=GRS80 +units=m +no_defs"
	EPSG3857PROJ = "+proj=merc +a=6378137 +b=6378137 +lat_ts=0.0 +lon_0=0.0 +x_0=0.0 +y_0=0 +k=1." +
		"0 +units=m +nadgrids=@null +no_defs"
	EPSG5186PROJ = "+proj=tmerc +lat_0=38 +lon_0=127 +k=1 +x_0=200000 +y_0=600000 +ellps=GRS80 +units=m +no_defs"
	EPSG5178PROJ = "+proj=tmerc +lat_0=38 +lon_0=127.5 +k=0.9996 +x_0=1000000 +y_0=2000000 +ellps=bessel +units=m +no_defs +towgs84=-115.80,474.99,674.11,1.16,-2.31,-1.63,6.43"
	EPSG5174PROJ = "+proj=tmerc +lat_0=38 +lon_0=127.0028902777778 +k=1 +x_0=200000 +y_0=500000 +ellps=bessel +units=m +no_defs +towgs84=-115.80,474.99,674.11,1.16,-2.31,-1.63,6.43"
)

var (
	inputDir           string
	outputDir          string
	inputEncoding      string
	inputCrs           string
	fileNameSubStrings string
	inputCrsObj        *proj.PJ
	inputCrsObj2       *proj.PJ
	outputCrsObj       *proj.PJ
)

func main() {
	flag.StringVar(&inputDir, "input", "../data/source", "input directory")
	flag.StringVar(&outputDir, "output", "../data/converted", "output directory")
	flag.StringVar(&inputEncoding, "input encoding", "euc-kr", "input encoding")
	flag.StringVar(&inputCrs, "input crs", "EPSG:5179", "input coordinate reference system")
	flag.StringVar(&fileNameSubStrings, "file name sub strings",
		"TL_SCCO_CTPRVN,TL_SCCO_SIG,TL_SCCO_EMD,TL_SCCO_LI,AL_D002", "substrings to filter shapefiles")
	flag.Parse()

	var err error
	if inputCrs != "EPSG:4326" {
		ctx := proj.NewContext()
		inputCrsObj, err = ctx.Create(EPSG5179PROJ)
		if err != nil {
			log.Fatalf("Error creating input CRS: %v", err)
		}

		inputCrsObj2, err = ctx.Create(EPSG5186PROJ)
		if err != nil {
			log.Fatalf("Error creating input CRS: %v", err)
		}

		outputCrsObj, err = ctx.Create(EPSG4326PROJ)
		if err != nil {
			log.Fatalf("Error creating output CRS: %v", err)
		}
	}

	err = convertShapefiles(inputDir, outputDir, inputEncoding, inputCrs, fileNameSubStrings)
	if err != nil {
		log.Fatalf("Error converting shapefiles: %v", err)
	}

	log.Println("conversion completed successfully.")
}

func getFilePaths(dir, ext, substrings string) ([]string, error) {
	subs := strings.Split(substrings, ",")
	var filePaths []string
	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return errors.Wrap(err, "")
		}

		if !info.IsDir() && filepath.Ext(info.Name()) == ext {
			exist := lo.CountBy(subs, func(sub string) bool {
				return strings.Contains(info.Name(), sub)
			})
			if exist > 0 {
				filePaths = append(filePaths, path)
			}
		}

		return nil
	})
	if err != nil {
		return nil, errors.Wrap(err, "")
	}

	return filePaths, nil
}

func convertShapefiles(inputDir, outputDir, inputEncoding,
	inputCrs, substrings string) error {
	log.Println(inputDir, outputDir, inputEncoding, inputCrs, substrings)
	err := os.RemoveAll(outputDir)
	if err != nil {
		return errors.Wrap(err, "")
	}

	filePaths, err := getFilePaths(inputDir, ".shp", substrings)
	if err != nil {
		return errors.Wrap(err, "")
	}

	err = os.MkdirAll(outputDir, os.ModePerm)
	if err != nil {
		return errors.Wrap(err, "")
	}

	for _, filePath := range filePaths {
		ss := strings.Split(filepath.Dir(filePath), "/")
		if len(ss) < 1 {
			return errors.New("invalid directory structure")
		}

		subOutputDir := fmt.Sprint(outputDir, "/", ss[len(ss)-1])
		if _, err := os.Stat(subOutputDir); os.IsNotExist(err) {
			err = os.MkdirAll(subOutputDir, os.ModePerm)
			if err != nil {
				return errors.Wrap(err, "failed to create output directory")
			}
		}

		outputFilePath := fmt.Sprint(subOutputDir, "/", filepath.Base(filePath))
		err = convertShapefile(filePath, outputFilePath, inputEncoding, inputCrs)
		if err != nil {
			return errors.Wrap(err, "failed to convert shapefile")
		}
	}

	return nil
}

func toUTF8(euckr string) (string, error) {
	if utf8.Valid([]byte(euckr)) {
		return euckr, nil
	}

	var buf bytes.Buffer
	wr := transform.NewWriter(&buf, korean.EUCKR.NewDecoder())
	defer wr.Close()

	_, err := wr.Write([]byte(euckr))
	if err != nil {
		return "", errors.Wrap(err, "failed to convert encoding")
	}

	return buf.String(), nil
}

func to4326(tx, ty float64, intputProj *proj.PJ) (float64, float64, error) {
	if intputProj == nil || outputCrsObj == nil {
		return tx, ty, nil
	}

	u, v, _, _, err := intputProj.Trans(proj.Inv, tx, ty, 0, 0)
	if err != nil {
		return tx, ty, errors.Wrap(err, "failed to transform coordinates")
	}

	x, y, _, _, err := outputCrsObj.Trans(proj.Fwd, u, v, 0, 0)
	if err != nil {
		return tx, ty, errors.Wrap(err, "failed to transform coordinates")
	}

	return proj.RadToDeg(x), proj.RadToDeg(y), nil
}

func convertShapefile(inputFilePath, outputFilePath, inputEncoding, inputCrs string) error {
	intputShape, err := shp.Open(inputFilePath)
	if err != nil {
		return errors.Wrap(err, "failed to open shapefile")
	}
	defer intputShape.Close()
	fields := intputShape.Fields()

	fileInfo, err := os.Stat(inputFilePath)
	if err != nil {
		return errors.Wrap(err, "failed to get file info")
	}

	outputFilePath = strings.ReplaceAll(outputFilePath, "(", "_")
	outputFilePath = strings.ReplaceAll(outputFilePath, ")", "")

	fileSizeMB := float64(fileInfo.Size()) / (1024 * 1024)
	fileIndex := -1
	const partitionPointSize = 600000
	const partitionFileSize = 12.0
	currentOutputFilePath := outputFilePath
	if strings.Contains(inputFilePath, "AL_D002") && fileSizeMB > partitionFileSize {
		fileIndex = 1
		currentOutputFilePath = fmt.Sprintf("%s_part%d.shp",
			strings.TrimSuffix(outputFilePath, ".shp"), fileIndex)
	}

	log.Println(inputFilePath, "->", currentOutputFilePath)

	outputShape, err := shp.Create(currentOutputFilePath, intputShape.GeometryType)
	if err != nil {
		return errors.Wrap(err, "failed to create output shapefile")
	}
	defer outputShape.Close()
	outputShape.SetFields(fields)

	inputProj := inputCrsObj
	if strings.Contains(inputFilePath, "AL_D002") {
		inputProj = inputCrsObj2
	}

	totalPointSize := 0
	for intputShape.Next() {
		n, p := intputShape.Shape()
		switch geom := p.(type) {
		case *shp.Polygon:
			for i := range geom.Points {
				lng, lat, err := to4326(geom.Points[i].X, geom.Points[i].Y, inputProj)
				if err != nil {
					return errors.Wrap(err, "failed to transform coordinates")
				}

				geom.Points[i].X = lng
				geom.Points[i].Y = lat
			}
			totalPointSize += len(geom.Points)
		default:
			return errors.Wrap(nil, "unsupported geometry type")
		}
		atribuite_index := outputShape.Write(p)

		for k, field := range fields {
			val := intputShape.ReadAttribute(n, k)
			if inputEncoding == "euc-kr" && field.Fieldtype == 'C' {
				val, err = toUTF8(val)
				if err != nil {
					return errors.Wrap(err, "failed to convert encoding")
				}
			}
			outputShape.WriteAttribute(int(atribuite_index), k, val)
		}

		if fileIndex > 0 && totalPointSize > partitionPointSize {
			outputShape.Close()
			if err != nil {
				return errors.Wrap(err, "failed to close output shapefile")
			}

			fileIndex++
			currentOutputFilePath = fmt.Sprintf("%s_part%d.shp",
				strings.TrimSuffix(outputFilePath, ".shp"), fileIndex)
			outputShape, err = shp.Create(currentOutputFilePath, intputShape.GeometryType)
			if err != nil {
				return errors.Wrap(err, "failed to create output shapefile")
			}
			defer outputShape.Close()
			outputShape.SetFields(fields)
			totalPointSize = 0

			log.Println("	", currentOutputFilePath)
		}
	}

	return nil
}
