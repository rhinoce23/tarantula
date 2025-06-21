const L = require('leaflet');
const shapefile = require('shapefile');
const { ipcRenderer } = require('electron');
require('leaflet-editable'); 
const toastr = require('toastr');
toastr.options = {
  "closeButton": true,
  "progressBar": true,
  "positionClass": "toast-top-right"
};
const fs = require('fs');
const { exec } = require('child_process');
const path = require('path');

const map = L.map('map', {editable: true}).setView([36.496723678, 127.279338989], 14);
L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {maxZoom: 30}).addTo(map);

ipcRenderer.send('renderer-ready');

let featureKey = '__featureKey__';
let shapefileGeojson = null;
let shapefilePath = null;
let editingFeature = null;
let shapefileGeojsonLayer = null;
let selectedFeatureLayer = null;

const latLngInput = document.getElementById('latLngInput');
latLngInput.addEventListener('keypress', goToLatLngOnEnter);

ipcRenderer.on('shapefile-path', (_, filePath) => {
  if (filePath) {
    shapefilePath = filePath
    dbfPath = path.join(path.dirname(shapefilePath), 
      path.basename(shapefilePath, '.shp') + '.dbf');
    shapefile.read(shapefilePath, dbfPath, {encoding: 'utf-8'})
      .then(geojson => {
        shapefileGeojson = geojson;
        let i = 1;
        shapefileGeojson.features.forEach((feature) => {
          feature.properties[featureKey] = i++;
        });
        shapefileGeojsonLayer = L.geoJSON(shapefileGeojson, {
          onEachFeature: (_, layer) => {
            layer.on('click', onFeatureClick);
          },
        }).addTo(map);
        map.fitBounds(L.geoJSON(shapefileGeojson).getBounds());
      })
      .catch(error => {
        toastr.error('error reading shapefile:', error);
      });
  } else {
    toastr.error('shapefile path not provided.');
  }
});


function onFeatureClick(event) {
  const feature = event.target.feature;

  if (selectedFeatureLayer) {
    map.removeLayer(selectedFeatureLayer);
  }

  editingFeature = feature;

  selectedFeatureLayer = L.geoJSON(feature, {
    style: {
      color: 'red',
      weight: 3,
    },
  }).addTo(map);

  selectedFeatureLayer.eachLayer((layer) => {
    if (layer instanceof L.Polygon) {
      layer.enableEdit(map);
      layer.on('editable:editing', (event) => {
        editingFeature = layer.toGeoJSON();
      });
    }
  });
}

function saveShapefile() {
  if (shapefileGeojson === null) {
    toastr.error('shapefile not loaded');
    return;
  }

  const clonedGeojson = JSON.parse(JSON.stringify(shapefileGeojson));
  clonedGeojson.features.forEach((feature) => {
    delete feature.properties[featureKey];
  });

  const match = shapefilePath.match(/([^\\/]+)\.([^.]+)$/);
  let fileName = "";
  if (match && match.length === 3) {
    fileName = match[1];
  } else {
    fileName = shapefilePath.match(/[^\\/]+$/)[0];
  }

  const jsonString = JSON.stringify(clonedGeojson, null, 2);
  const updatedPath = path.dirname(shapefilePath);
  try {
    fs.writeFileSync(`${updatedPath}/${fileName}.geojson`, jsonString, 'utf-8');
  } catch (error) {
    toastr.error(`${updatedPath}/${fileName}.geojson error saving`, error);
  }

  const toGeojsonCommand = `ogr2ogr -f "ESRI Shapefile" \
    ${updatedPath}/${fileName}.shp ${updatedPath}/${fileName}.geojson \
    -s_srs EPSG:4326 -t_srs EPSG:4326 -lco ENCODING=UTF-8`;
  executeCommand(toGeojsonCommand).then(() => {
    toastr.success(`${updatedPath}/${fileName}.shp saved`);
  }
  ).catch((error) => {
    toastr.error(`${updatedPath}/${fileName}.shp error saving`, error);
  });
}

async function executeCommand(command) {
  return new Promise((resolve, reject) => {
    exec(command, (error, stdout, stderr) => {
      if (error) {
          console.log(error);
          reject(error);
          return;
      }
      resolve();
    });
  });
}

document.addEventListener('keydown', (event) => {
  if (event.code === 'KeyS') {
    saveShapefile();
  } else if (event.code === 'KeyX' && selectedFeatureLayer) {
    map.removeLayer(selectedFeatureLayer);
    editingFeature = null;
  } else if (event.code === 'KeyU') {
    if (editingFeature === null) {
      toastr.error('no feature selected.');
      return;
    }

    const featureIndex = shapefileGeojson.features.findIndex(feature => {
      return feature.properties && editingFeature.properties 
        && feature.properties[featureKey] === editingFeature.properties[featureKey];
    });

    if (featureIndex === -1) {
      toastr.error('feature not found.');
      return;
    }

    shapefileGeojson.features[featureIndex] = editingFeature;
    map.removeLayer(selectedFeatureLayer);
    editingFeature = null;

    map.removeLayer(shapefileGeojsonLayer);
    shapefileGeojsonLayer = L.geoJSON(shapefileGeojson, {
      onEachFeature: (_, layer) => {
        layer.on('click', onFeatureClick);
      },
    }).addTo(map);
  }
});

function goToLatLngOnEnter(event) {
  if (event.key === 'Enter') {
    const inputElement = event.target;
    const coordinates = inputElement.value.trim().split(',');

    if (coordinates.length === 2) {
      const lng = parseFloat(coordinates[0]);
      const lat = parseFloat(coordinates[1]);

      if (!isNaN(lat) && !isNaN(lng)) {
        map.setView([lat, lng], 25); 
      } else {
        toastr.error('please enter valid latitude and longitude ex) 126.939056574,36.218311760');
      }
    } else {
      toastr.error('please enter latitude and longitude separated by a comma ex) 126.939056574,36.218311760');
    }
  }
}

const addressInfoDiv = document.getElementById('address-info');

map.on('moveend', () => {
  const center = map.getCenter();
  fetch(`http://localhost:8080/tarantula?lon=${center.lng}&lat=${center.lat}`)
    .then(res => res.json())
    .then(data => {
      if (!Array.isArray(data)) {
        return;
      }

      const lnglats = data.find(info =>
        Array.isArray(info.lnglats) &&
        info.lnglats.length > 2
      );

      const latlngs = lnglats.lnglats.map(ll => [ll[1], ll[0]]);

      if (window.centerPolygonLayer) {
        map.removeLayer(window.centerPolygonLayer);
        window.centerPolygonLayer = null;
      }

      window.centerPolygonLayer = L.polygon(latlngs, {
        color: 'blue',
        weight: 2,
        fillOpacity: 0.1
      }).addTo(map);

      let infos = data.map((info, i) => {
        if (i === 0) {
          return `${info.district} [${info.level}] ${info.name}`;
        }
        return `[${info.level}] ${info.name}`;
      });
      
      addressInfoDiv.textContent = infos.join(' / ');
    })
    .catch(err => {
      addressInfoDiv.textContent = err;
    });
});

