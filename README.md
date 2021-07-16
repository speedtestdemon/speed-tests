# How to run speed test.

## the image to use

The image you should probably use is: cf_219kb.png

cf_219kb.png is an image that won't be compressed by Jetpack Wordpress. So,
uploading that image to both Jetpack and CloudFront CDN will result in the same
image being downloaded.

curl-format.txt is how to run the speed test.

Here is a shell script of how the speed test is run:

## the curl command

```
speedtest() {
  curl -w "@curl-format.txt" -o tmp -s $@
}
```

Please note that you do NOT want to `-o /dev/null`, as this will make curl
will cleverly skip the data transfer (i.e. download) phase. Which will throw
off your speed test measurement. So it is important to have `-o tmp` to
actually download the file.

## the measurement

the very first curl you should copy-paste into your notes. This curl is very
interesting to measure because the 1st curl is always the slowest. This is
usually the biggest variance between the CDNs. The first curl is an indicator
of how fast the CDN loads for 1st-time visitors.

the 2nd measurement is doing at least 10 subsequent curls, and taking the average.
This measures how well a CDN caches content for multiple page visitors.
# The script

## Installation:
The script uses the pycurl module. To install it run:

```
$ pip3 install pycurl
```

If the installation fails you may need to install these dependencies:
- libssl-dev 
- libcurl4-openssl-dev
On Linux:

```
$ sudo apt-get install libssl-dev libcurl4-openssl-dev
```

## Testing:

To run it just run:

```
$python3 script.py < url >
```
### Example:
```
$ python3 script.py 'https://d3va53q3li7xt1.cloudfront.net/wp-content/uploads/2021/05/shoeb-1024x576.png'
```
#### Output:
```
10 requests done. Average:
time_namelookup: 0.004422
time_connect: 0.014404
time_appconnect: 0.036274900000000006
time_pretransfer: 0.03636879999999999
time_redirect: 0.0
time_starttransfer: 0.049656500000000006
time_total: 0.0496897
```

# The other files

```
cf.png is 703KB.
wp.png is 329KB.
```

Jetpack Wordpress compressed cf.png down to wp.png, which is why Jetpack won
the initial speed test.
