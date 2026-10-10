import * as echarts from "echarts/core";
import {
  MapChart,
  ScatterChart,
  BarChart,
  CustomChart,
  LineChart,
  LinesChart,
} from "echarts/charts";
import {
  GeoComponent,
  TooltipComponent,
  GridComponent,
  VisualMapComponent,
  MarkLineComponent,
  AriaComponent,
} from "echarts/components";
import { SVGRenderer } from "echarts/renderers";
import { LabelLayout } from "echarts/features";
echarts.use([
  MapChart,
  ScatterChart,
  BarChart,
  CustomChart,
  LineChart,
  LinesChart,
  GeoComponent,
  TooltipComponent,
  GridComponent,
  VisualMapComponent,
  MarkLineComponent,
  AriaComponent,
  SVGRenderer,
  LabelLayout,
]);
export { echarts };
