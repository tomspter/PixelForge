#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include <opencv2/photo.hpp>

int main() {
  cv::Mat source(32, 32, CV_8UC3, cv::Scalar(210, 208, 205));
  cv::Mat mask = cv::Mat::zeros(source.size(), CV_8UC1);
  cv::rectangle(source, cv::Rect(14, 8, 4, 16), cv::Scalar(12, 12, 12), cv::FILLED);
  cv::rectangle(mask, cv::Rect(14, 8, 4, 16), cv::Scalar(255), cv::FILLED);

  cv::Mat result;
  cv::inpaint(source, mask, result, 3.0, cv::INPAINT_TELEA);
  if (result.empty() || result.size() != source.size() || result.type() != source.type()) {
    return 1;
  }

  const cv::Vec3b repaired = result.at<cv::Vec3b>(16, 16);
  return repaired[0] > 150 && repaired[1] > 150 && repaired[2] > 150 ? 0 : 2;
}
