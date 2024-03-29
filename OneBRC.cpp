#include <boost/interprocess/file_mapping.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/interprocess/containers/map.hpp>
#include <boost/interprocess/allocators/allocator.hpp>
#include <functional>
#include <utility>
#include <iostream>
#include <unordered_map>
#include <unordered_set>
#include <string>
#include <boost/filesystem.hpp>
#include <thread>
#include <iomanip>
#include <vector>
#include <algorithm>
#include <string_view>
#include <city.h>
#include <chrono>

const size_t overlap = 128; // 100 + ';' + optional negative sign + 2 decimal places + '.' + decimal place + '\n' will fit below 128 bytes

struct CityKey {
    std::string_view c;
    uint64_t hash_value = 0;

    bool operator==(const CityKey& other) const  noexcept
    {
        return c == other.c;
    }

    std::string to_string() const noexcept
    {
        return std::string{c};
    }
};

struct CityKeyHash
{
    std::size_t operator()(const CityKey& city) const noexcept
    {
        return city.hash_value;
    }
};

struct Weather {
    int16_t minTemp = 0;
    int16_t maxTemp = 0;
    int32_t netTemp = 0;
    uint32_t count = 0;

    void setTemperature(int16_t temp)
    {
        minTemp = std::min(temp, minTemp);
        maxTemp = std::max(maxTemp, temp);
        netTemp += temp;
        count++;
    }

    void inplaceMerge(const Weather& other) {
        minTemp = std::min(minTemp, other.minTemp);
		maxTemp = std::max(maxTemp, other.maxTemp);
		netTemp += other.netTemp;
		count += other.count;
    }

    void print(std::ostream& out) const
    {
        float mint = static_cast<float>(minTemp) / 10.0;
        float maxt = static_cast<float>(maxTemp) / 10.0;

        float average = 0;
        if (count != 0)
            average = static_cast<float>(netTemp) / (10.0 * count);

        out << "=" << mint << "/" << std::fixed << std::setprecision(1)  << average << "/" << maxt;
    }
};


class WeatherBatch {
    std::unordered_map<CityKey, Weather, CityKeyHash> cityWeatherMap;

public:
    WeatherBatch() {
        cityWeatherMap.reserve(200000);
    }
    WeatherBatch(const WeatherBatch& other) = default;
    WeatherBatch(WeatherBatch&& other) = default;
    WeatherBatch& operator=(const WeatherBatch& other) = default;
    WeatherBatch& operator=(WeatherBatch&& other) = default;
    ~WeatherBatch() = default;

    inline void addBatch(const char* data, size_t size)
    {
        size_t i = 0;
        while (i < size) {
            const char* city_start = &data[i];
            const char* semi_colon = static_cast<const char*>(std::memchr(&data[i], ';', overlap));
            size_t city_size = semi_colon - city_start;
            std::string_view c {city_start, city_size};
            i = i + city_size;
            CityKey city;
            city.c = std::move(c);
            city.hash_value = CityHash64(city_start, city_size);
            i++;
            int16_t temperature = 0;
            int16_t sign = 1;
            if(data[i] == '-') {
                sign = -1;
                i++;
            }

            while (i < size && data[i] != '\n') {
                if(data[i] != '.') {
                    temperature = temperature * 10 + (data[i] - '0');
                }
                i++;
            }
            cityWeatherMap[city].setTemperature(temperature * sign);
            i++;
        }
    }

    void mergeTo(WeatherBatch& target) const  {
        for (auto iter = cityWeatherMap.begin(); iter != cityWeatherMap.end(); ++iter) {
            target.cityWeatherMap[iter->first].inplaceMerge(iter->second);
        }
    }

    void print(std::ostream& out) const
    {
        std::vector<std::string> cities;
        std::unordered_map<std::string, Weather> table;
        cities.reserve(10000);
        for (auto iter= cityWeatherMap.cbegin(); iter != cityWeatherMap.end(); ++iter) {
            auto city = iter->first.to_string();
            cities.push_back(city);
            table[city] = iter->second;
        }
        std::sort(cities.begin(), cities.end());

        bool isFirst = true;
        out << "{" ;
        for (const std::string& city : cities) {
            if (!isFirst) {
                out << ", " ;
            }
            isFirst = false;
			out << city;
            auto find = table.find(city);
            if (find != table.end()) {
                find->second.print(out);
            }
		}
        out << '}' << std::endl;
	}
};

class Worker {
    size_t start = 0;;
    size_t end = 0;
    const char* root = nullptr;
    WeatherBatch workerData;
    bool skipBegin;
public:
    Worker(const char* ptr,  size_t start, size_t end)
        : start(start)
        , end(end)
        , root(ptr)
        , skipBegin(start != 0)
    {
 
	}

    
    void execute()
    {
        const char* data =  &root[start]; 

        auto regionSize = end - start;
        if (regionSize == 0) {
            return;
        }
        workerData.addBatch(data, regionSize);
	}

    WeatherBatch getData() const
    {
		return workerData;
	}

    void collect(WeatherBatch& target) const
    {
        workerData.mergeTo(target);
    }
};

size_t nextEnd(boost::interprocess::file_mapping& file, size_t start)
{
    boost::interprocess::mapped_region region(file, boost::interprocess::read_only, start, overlap);
    const char* data = static_cast<const char*>(region.get_address());
    size_t firstSlashN = 0;
    for (firstSlashN = 0; firstSlashN < overlap; ++firstSlashN) {
        if (data[firstSlashN] == '\n') {
            break;
        }
    }
    return start + firstSlashN;
}

int main(int argc, char** argv)
{
    if (argc < 2) {
        std::cout << " Need file Name " << std::endl ;
        return 1;
    }
    auto start_time = std::chrono::high_resolution_clock::now();
    std::string fname = argv[1];
    size_t fileSize = boost::filesystem::file_size(fname);
    size_t workerSize = fileSize / 32;

    boost::interprocess::file_mapping file(fname.c_str(), boost::interprocess::read_only);
    boost::interprocess::mapped_region region(file, boost::interprocess::read_only);
    const char *data = static_cast<const char*>(region.get_address());
    size_t numThreads = 32;
    std::vector<Worker*> workerPtrs;
    size_t start = 0;
    size_t prevEnd = nextEnd(file, workerSize);
    workerPtrs.push_back(new Worker(data, start, prevEnd));
    for (size_t i = 1; i < numThreads - 1; ++i) {
        size_t currentEnd = nextEnd(file, prevEnd + workerSize);
        workerPtrs.push_back(new Worker(data , prevEnd + 1, currentEnd));
        prevEnd = currentEnd;
    }
    workerPtrs.push_back(new Worker(data , prevEnd + 1, fileSize));
    std::vector<std::thread> threads;
    for (size_t i = 0; i < numThreads; ++i) {
        std::thread t { &Worker::execute, workerPtrs[i]};
        threads.push_back(std::move(t));
    }

    for(auto iter = threads.begin(); iter != threads.end(); iter++) {
        iter->join();
    }
   
    WeatherBatch result;
    for(auto iter = workerPtrs.begin(); iter != workerPtrs.end(); ++iter) {
        (*iter)->collect(result);
    }
    for(auto p : workerPtrs) {
        delete p;
    }
    workerPtrs.clear();
    result.print(std::cout);
    auto stop_time = std::chrono::high_resolution_clock::now();
    auto time_span = std::chrono::duration_cast<std::chrono::milliseconds>(stop_time - start_time);
    std::cout << time_span <<  std::endl;
    return 0;
}
