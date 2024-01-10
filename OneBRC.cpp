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

const size_t overlap = 128; // 100 + ';' + optional negative sign + 2 decimal places + '.' + decimal place + '\n' will fit below 128 bytes
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
    std::unordered_map<std::string, Weather> cityWeatherMap;

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
            size_t city_size = 0;
            const char* city_start = &data[i];
            while (i < size &&  data[i] != ';') {
                city_size++;
                i++;
            }
            std::string city{city_start, city_size};
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
        cities.reserve(10000);
        for (auto iter= cityWeatherMap.cbegin(); iter != cityWeatherMap.end(); ++iter) {
            cities.push_back(iter->first);
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
            auto find = cityWeatherMap.find(city);
            if (find != cityWeatherMap.end()) {
                find->second.print(out);
            }
		}
        out << '}' << std::endl;
	}
};

class Worker {
    size_t start;
    size_t end;
    boost::interprocess::file_mapping& file;
    WeatherBatch workerData;
    bool skipBegin;
public:
    Worker(boost::interprocess::file_mapping& f , size_t start, size_t end)
        : start(start)
        , end(end)
        , file(f)
        , skipBegin(start != 0)
    {
 
	}

    
    void execute()
    {
        boost::interprocess::mapped_region region(file, boost::interprocess::read_only, start , end - start);
        const char* data = static_cast<const char*>(region.get_address());
        size_t last_slashn = 0;

        auto regionSize = region.get_size();
        if (regionSize == 0) {
            return;
        }
        workerData.addBatch(&data[last_slashn], regionSize);
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
    std::string fname = argv[1];
    size_t fileSize = boost::filesystem::file_size(fname);
    //size_t worker_size = fileSize / 10;
    size_t workerSize = fileSize / 8;

    boost::interprocess::file_mapping file(fname.c_str(), boost::interprocess::read_only);
    auto w1End = nextEnd(file, workerSize);
    Worker w1(file, 0,  w1End);
    std::thread t1 (&Worker::execute, &w1);
    auto w2End = nextEnd(file, w1End + workerSize);
    Worker w2(file, w1End + 1,   w2End);
    std::thread t2 (&Worker::execute, &w2);
    auto w3End = nextEnd(file, w2End + workerSize);
    Worker w3(file, w2End + 1, w3End );
    std::thread t3 (&Worker::execute, &w3);
    auto w4End = nextEnd(file, w3End + workerSize);
    Worker w4(file, w3End + 1,  w4End);
    std::thread t4 (&Worker::execute, &w4);
	auto w5End = nextEnd(file, w4End + workerSize);
    Worker w5(file, w4End + 1,  w5End);
    std::thread t5 (&Worker::execute, &w5);
    auto w6End = nextEnd(file, w5End + workerSize);
    Worker w6(file, w5End + 1,  w6End);
    std::thread t6 (&Worker::execute, &w6);
    auto w7End = nextEnd(file, w6End + workerSize);
    Worker w7(file, w6End + 1,  w7End);
    std::thread t7 (&Worker::execute, &w7);
    Worker w8(file, w7End + 1,  fileSize);
    std::thread t8 (&Worker::execute, &w8);

    t1.join();
    t2.join();
    t3.join();
    t4.join();
    t5.join();
    t6.join();
    t7.join();
    t8.join();
   
    WeatherBatch result;
    w1.collect(result);
    w2.collect(result);
    w3.collect(result);
    w4.collect(result);
    w5.collect(result);
    w6.collect(result);
    w7.collect(result);
    w8.collect(result);
    result.print(std::cout);
    return 0;
}
