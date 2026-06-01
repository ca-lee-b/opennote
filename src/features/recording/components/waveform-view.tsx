import { motion } from "motion/react";
import { useMemo } from "react";

const BAR_COUNT = 40;
const BAR_MIN_HEIGHT = 4;
const BAR_MAX_HEIGHT = 36;
const BAR_WIDTH = 3;
const BAR_RADIUS = 1.5;

interface WaveformViewProps {
  audioLevel: number;
  isActive: boolean;
}

export function WaveformView({ audioLevel, isActive }: WaveformViewProps) {
  const bars = useMemo(() => {
    return Array.from({ length: BAR_COUNT }, (_, i) => {
      const centerDist = Math.abs(i - BAR_COUNT / 2) / (BAR_COUNT / 2);
      const baseScale = 1 - centerDist * 0.6;
      return { id: i, baseScale };
    });
  }, []);

  return (
    <div
      className="flex items-end justify-center gap-[2px]"
      style={{ height: BAR_MAX_HEIGHT }}
    >
      {bars.map((bar) => (
        <motion.div
          animate={{
            height: isActive
              ? BAR_MIN_HEIGHT +
                audioLevel * (BAR_MAX_HEIGHT - BAR_MIN_HEIGHT) * bar.baseScale
              : BAR_MIN_HEIGHT,
          }}
          className="bg-foreground/70"
          initial={{ height: BAR_MIN_HEIGHT }}
          key={bar.id}
          style={{
            width: BAR_WIDTH,
            borderRadius: BAR_RADIUS,
          }}
          transition={{
            type: "spring",
            stiffness: 400,
            damping: 30,
            mass: 0.5,
          }}
        />
      ))}
    </div>
  );
}
