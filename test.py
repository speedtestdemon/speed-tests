import pycurl
import io
import sys
import time
 
if len(sys.argv) != 2:
    raise ValueError('Please provide a url')
url = sys.argv[1]

headers = io.BytesIO()

def curl(url):
    global headers
    headers = io.BytesIO()
    buf = io.BytesIO() # We need to measure download time.
    c = pycurl.Curl()
    c.setopt(c.URL, url)
    c.setopt(c.HEADERFUNCTION, headers.write)
    c.setopt(c.WRITEFUNCTION, buf.write)
    c.perform()
    # Sanity check that we got a download.
    # print('download size', len(buf.getvalue()))
    return c

def printresults(results):
    print('time_namelookup: {:.20f}'.format(results[0]))
    print('time_connect: {:.20f}'.format(results[1]))
    print('time_appconnect: {:.20f}'.format(results[2]))
    print('time_pretransfer: {:.20f}'.format(results[3]))
    print('time_redirect: {:.20f}'.format(results[4]))
    print('time_starttransfer: {:.20f}'.format(results[5]))
    print('time to download: {:.20f}'.format(results[6]))
    print('time_total: {:.20f}'.format(results[7]))

# Turn data from cumulative seconds to individual seconds
def fixdata(D):
    # return D # if you want to debug the raw curl numbers.
    i = len(D) - 3
    cumulativetime = D[ i+1 ]
    cur_i = i+1 # we start setting values at 2nd to last num.
    while i >= 0:
        if D[i] > 0:
            D[cur_i] = cumulativetime - D[i] # calculates new value for cur_i
            cumulativetime = D[i] # updates the new cumulative time.
            cur_i = i
        i -= 1
    return D

# This is done for "cold cache test" and "warm cache test".
def singlecurltest(url):
    c = curl(url)
    print('Got headers:')
    print(headers.getvalue().decode("utf-8"))
    data = [c.getinfo(c.NAMELOOKUP_TIME),
            c.getinfo(c.CONNECT_TIME),
            c.getinfo(c.APPCONNECT_TIME),
            c.getinfo(c.PRETRANSFER_TIME),
            c.getinfo(c.REDIRECT_TIME),
            c.getinfo(c.STARTTRANSFER_TIME),
            c.getinfo(c.TOTAL_TIME),
            c.getinfo(c.TOTAL_TIME) ]
    data = fixdata(data)
    printresults(data)

print('-------------------------------------------------------------')
print('Testing "Cold cache speed"')
print('-------------------------------------------------------------')
singlecurltest(url)

print('-------------------------------------------------------------')
print('Testing "Hot cache speed"')
print('-------------------------------------------------------------')

# url = "https://d3va53q3li7xt1.cloudfront.net/wp-content/uploads/2021/05/shoeb-1024x576.png"
n = 10

responses = []
for i in range(n):
    c = curl(url)
    data = [c.getinfo(c.NAMELOOKUP_TIME),
        c.getinfo(c.CONNECT_TIME),
        c.getinfo(c.APPCONNECT_TIME),
        c.getinfo(c.PRETRANSFER_TIME),
        c.getinfo(c.REDIRECT_TIME),
        c.getinfo(c.STARTTRANSFER_TIME),
        c.getinfo(c.TOTAL_TIME),
        c.getinfo(c.TOTAL_TIME) ]
    data = fixdata(data)

    responses.append(data)
    c.close()


total_sum = [0,0,0,0,0,0,0,0]
for response in responses:
    for i in range(len(response)):
       total_sum[i] = total_sum[i] + response[i] 
        
results = [0,0,0,0,0,0,0,0]
for i in range(len(total_sum)):
    results[i] = total_sum[i]/n

print('{0} requests done. Average:'.format(n))
printresults(results)


print('-------------------------------------------------------------')
print('Testing "Warm cache speed"')
print('-------------------------------------------------------------')
print('Sleeping for 0.5 hr to move cache from hot to warm')
time.sleep(1800) # 1800 seconds == 0.5 hr
singlecurltest(url)
