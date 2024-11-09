/*
 * @Author: Jiang Han
 * @Date: 2024-09-29 21:11:37
 * @Description: 
 */
import React, { useEffect, useState } from 'react';
import { Line } from 'react-chartjs-2';
import { Chart as ChartJS, CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend } from 'chart.js';

// Register the necessary scales and components for Chart.js
ChartJS.register(
  CategoryScale,   // for the X-axis
  LinearScale,     // for the Y-axis
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
);

const RealTimeChart = () => {
  const [blockHeights, setBlockHeights] = useState<number[]>([]);
  const [prices, setPrices] = useState<number[]>([]);
  const [timestamps, setTimestamps] = useState<string[]>([]);

  // Fetch all data from the server initially
  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await fetch('http://34.171.207.200:5000/all-data');
        const data = await response.json();

        setBlockHeights(data.map((entry: any) => entry.block_height));
        setPrices(data.map((entry: any) => entry.price));
        setTimestamps(data.map((entry: any) => entry.timestamp));
      } catch (error) {
        console.error('Error fetching historical data:', error);
      }
    };
    fetchData();
  }, []);

  // Setup WebSocket for real-time updates
  useEffect(() => {
    const socket = new WebSocket('ws://34.171.207.200/:5001');
    socket.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setBlockHeights((prev) => [...prev, data.block_height]);
      setPrices((prev) => [...prev, data.price]);
      setTimestamps((prev) => [...prev, new Date().toLocaleTimeString()]);
    };

    return () => socket.close();
  }, []);

  const chartData = {
    labels: timestamps,
    datasets: [
      {
        label: 'Block Height',
        data: blockHeights,
        borderColor: 'blue',
        fill: false,
      },
      {
        label: 'Bitcoin Price (USD)',
        data: prices,
        borderColor: 'green',
        fill: false,
      },
    ],
  };

  return <Line data={chartData} />;
};

export default RealTimeChart;
