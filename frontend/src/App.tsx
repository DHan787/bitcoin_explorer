/*
 * @Author: Jiang Han
 * @Date: 2024-09-29 21:11:37
 * @Description: 
 */
import React, { useEffect, useState } from 'react';
import axios from 'axios';

interface BlockData {
    block_height: number;
    timestamp: string;
}

const App: React.FC = () => {
    const [blockHeight, setBlockHeight] = useState<BlockData | null>(null);

    useEffect(() => {
        const fetchData = async () => {
            try {
                const response = await axios.get<BlockData>('http://localhost:5000/block-height');
                setBlockHeight(response.data);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };

        fetchData();
        const interval = setInterval(fetchData, 60000); // Poll every 60 seconds

        return () => clearInterval(interval);
    }, []);

    return (
        <div>
            <h1>Bitcoin Explorer</h1>
            {blockHeight ? (
                <div>
                    <p>Latest Block Height: {blockHeight.block_height}</p>
                    <p>Timestamp: {new Date(blockHeight.timestamp).toLocaleString()}</p>
                </div>
            ) : (
                <p>Loading...</p>
            )}
        </div>
    );
};

export default App;
