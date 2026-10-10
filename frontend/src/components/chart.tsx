"use client";
import { useEffect, useRef, type RefObject } from "react";
import type { EChartsType } from "echarts/core";
import type { EChartsOption } from "echarts";
import { echarts } from "@/lib/echarts";
export type ChartClick = {
  name: string;
  seriesName?: string;
  dataIndex: number;
  data?: { id?: string };
};
export default function Chart({
  option,
  label,
  height = 200,
  onClick,
  chartRef,
  onReady,
}: {
  option: EChartsOption;
  label: string;
  height?: number;
  onClick?: (event: ChartClick) => void;
  chartRef?: RefObject<EChartsType | null>;
  onReady?: (chart: EChartsType | null) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const instance = useRef<EChartsType | null>(null);
  const click = useRef(onClick);
  useEffect(() => {
    click.current = onClick;
  }, [onClick]);
  const latestOption = useRef(option);
  useEffect(() => {
    latestOption.current = option;
    const chart = instance.current;
    if (!chart) return;
    // Selecting a map point must not discard the user's zoom or pan.
    const previous = (
      chart.getOption().geo as
        { map?: string; center?: number[]; zoom?: number }[] | undefined
    )?.[0];
    const geo =
      option.geo && !Array.isArray(option.geo) ? option.geo : undefined;
    const next =
      geo && previous && previous.map === geo.map
        ? {
            ...option,
            geo: { ...geo, center: previous.center, zoom: previous.zoom },
          }
        : option;
    chart.setOption(withDefaults(next), { notMerge: true });
  }, [option]);
  useEffect(() => {
    const element = container.current;
    if (!element) return;
    let chart: EChartsType | null = null;
    function resizeOrInitialize() {
      if (
        !element?.isConnected ||
        element.clientWidth === 0 ||
        element.clientHeight === 0
      )
        return;
      if (!chart) {
        chart = echarts.init(element, undefined, { renderer: "svg" });
        instance.current = chart;
        if (chartRef) chartRef.current = chart;
        chart.on("click", (p) => click.current?.(p as ChartClick));
        chart.setOption(withDefaults(latestOption.current), { notMerge: true });
        onReady?.(chart);
      } else if (!chart.isDisposed()) {
        chart.resize();
      }
    }
    // Hidden responsive columns and collapsed details have no measurable size.
    // Initialize only after they become visible, using the latest data.
    const observer = new ResizeObserver(resizeOrInitialize);
    observer.observe(element);
    resizeOrInitialize();
    return () => {
      observer.disconnect();
      chart?.dispose();
      instance.current = null;
      if (chartRef) chartRef.current = null;
      onReady?.(null);
    };
  }, [chartRef, onReady]);
  return (
    <div
      ref={container}
      className="echart"
      style={{ height }}
      role="img"
      aria-label={label}
    />
  );
}

function withDefaults(option: EChartsOption): EChartsOption {
  return {
    animation: false,
    textStyle: {
      fontFamily: '"Helvetica Neue", Arial, sans-serif',
      color: "#606870",
    },
    ...option,
  };
}
