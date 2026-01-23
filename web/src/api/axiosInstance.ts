//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


import { getToken } from "@/stores/authStore";
import axios from "axios";

// Create an Axios instance
const baseURL = process.env.NODE_ENV === "production"
  ? "/" // Production: relative to the current domain
  : "http://localhost:15630"; // Development: Poem's backend server

const axiosInstance = axios.create({
  baseURL,
  timeout: 60000, // Timeout in milliseconds
  headers: {
    "Content-Type": "application/json",  // Explicitly setting Content-Type to application/json
  },
});

// Add a request interceptor to include the access token in headers
axiosInstance.interceptors.request.use(
  (config) => {
    const stored = getToken(); // Retrieve access token from localStorage
    if (stored) {
      config.headers.Authorization = `Bearer ${stored.accessToken}`;
    }
    return config;
  },
  (error) => {
    // Handle request errors
    return Promise.reject(error);
  }
);

// Add a response interceptor (optional)
axiosInstance.interceptors.response.use(
  (response) => response,
  (error) => {
    // Handle response errors
    //console.error("API error:", error.response?.data || error.message);
    return Promise.reject(error);
  }
);

export default axiosInstance;
