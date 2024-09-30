/*
 * @Author: Jiang Han
 * @Date: 2024-09-29 20:13:34
 * @Description: 
 */
const express = require('express');
const { Pool } = require('pg');
const cors = require('cors');

const app = express();
app.use(cors());

const pool = new Pool({
    user: 'postgres',
    host: 'postgres',
    database: 'bitcoin_explorer',
    password: 'postgre',
    port: 5432,
});

app.get('/block-height', async (req, res) => {
    try {
        const result = await pool.query('SELECT * FROM block_data ORDER BY id DESC LIMIT 1');
        if (result.rows.length > 0) {
            res.json(result.rows[0]);
        } else {
            res.status(404).json({ error: 'No data found' });
        }
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

app.listen(5000, () => {
    console.log('Server running on http://localhost:5000');
});
